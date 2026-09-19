#!/usr/bin/env python3
"""
Tests de l'interface fixe (HUD) dans un vrai pseudo-terminal.

Le jeu est lancé dans un pty de taille connue ; sa sortie passe par un mini-émulateur de
terminal (curseur, région de défilement, effacements, sauvegarde du curseur) qui reconstitue
l'écran tel que le verrait le joueur. On vérifie donc ce qui est *affiché*, pas seulement
les séquences émises. Aucune dépendance hors bibliothèque standard.

    NEON_HACK_BIN=./neon_hack tests/e2e/pty_hud.py
"""

import fcntl
import os
import pty
import re
import select
import signal
import struct
import sys
import termios
import time
import unicodedata

BIN = os.environ.get("NEON_HACK_BIN", "./neon_hack")
PROMPT = "neon-terminal] $ "
ANSI = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]")


class Screen:
    """Émulateur minimal : ce que le jeu utilise (et un peu plus), rien d'autre."""

    def __init__(self, rows, cols):
        self.rows, self.cols = rows, cols
        self.cells = [[" "] * cols for _ in range(rows)]
        self.r = self.c = 0
        self.top, self.bottom = 0, rows - 1
        self.saved = (0, 0)
        self.autowrap = True
        self.pending_wrap = False
        self.region_resets = 0
        self.scrolled_off = []  # lignes sorties par le haut de la région (l'« historique »)

    def resize(self, rows, cols):
        """Comme un vrai terminal : le contenu est conservé, la région de défilement remise à zéro."""
        old = self.cells
        self.cells = [[" "] * cols for _ in range(rows)]
        for r in range(min(rows, len(old))):
            for c in range(min(cols, len(old[r]))):
                self.cells[r][c] = old[r][c]
        self.rows, self.cols = rows, cols
        self.top, self.bottom = 0, rows - 1
        self.r, self.c = min(self.r, rows - 1), min(self.c, cols - 1)
        self.pending_wrap = False

    # -- mouvements --
    def _scroll_up(self):
        self.scrolled_off.append("".join(self.cells[self.top]).rstrip())
        del self.cells[self.top]
        self.cells.insert(self.bottom, [" "] * self.cols)

    def _linefeed(self):
        if self.r == self.bottom:
            self._scroll_up()
        elif self.r < self.rows - 1:
            self.r += 1

    def _put(self, ch):
        w = 2 if unicodedata.east_asian_width(ch) in ("W", "F") else 1
        if unicodedata.combining(ch):
            return
        if self.pending_wrap and self.autowrap:
            self.c = 0
            self._linefeed()
        self.pending_wrap = False
        if self.c + w > self.cols:
            self.c = 0
            self._linefeed()
        self.cells[self.r][self.c] = ch
        if w == 2 and self.c + 1 < self.cols:
            self.cells[self.r][self.c + 1] = ""
        self.c += w
        if self.c >= self.cols:
            self.c = self.cols - 1
            self.pending_wrap = True

    # -- séquences --
    def feed(self, text):
        i, n = 0, len(text)
        while i < n:
            ch = text[i]
            if ch == "\x1b":
                i = self._escape(text, i)
                continue
            if ch == "\n":
                self.pending_wrap = False
                self._linefeed()
            elif ch == "\r":
                self.pending_wrap = False
                self.c = 0
            elif ch == "\b":
                self.c = max(0, self.c - 1)
            elif ch >= " ":
                self._put(ch)
            i += 1

    def _escape(self, t, i):
        if i + 1 >= len(t):
            return i + 1
        nxt = t[i + 1]
        if nxt == "7":
            self.saved = (self.r, self.c)
            return i + 2
        if nxt == "8":
            self.r, self.c = self.saved
            self.pending_wrap = False
            return i + 2
        if nxt != "[":
            return i + 2
        m = re.compile(r"[?0-9;]*[ -/]*[@-~]").match(t, i + 2)
        if not m:
            return i + 2
        seq = m.group(0)
        self._csi(seq[-1], seq[:-1])
        return m.end()

    def _csi(self, final, params):
        private = params.startswith("?")
        nums = [int(p) if p.isdigit() else 0 for p in params.lstrip("?").split(";")] if params else []
        arg = lambda k, d: nums[k] if len(nums) > k and nums[k] > 0 else d
        self.pending_wrap = False
        if final == "H" or final == "f":
            self.r = min(self.rows - 1, arg(0, 1) - 1)
            self.c = min(self.cols - 1, arg(1, 1) - 1)
        elif final == "r":
            if not nums:
                self.top, self.bottom = 0, self.rows - 1
                self.region_resets += 1
            else:
                self.top, self.bottom = arg(0, 1) - 1, min(self.rows, arg(1, self.rows)) - 1
            self.r, self.c = 0, 0
        elif final == "K":
            mode = nums[0] if nums else 0
            row = self.cells[self.r]
            lo, hi = {0: (self.c, self.cols), 1: (0, self.c + 1), 2: (0, self.cols)}.get(mode, (0, 0))
            for k in range(lo, hi):
                row[k] = " "
        elif final == "J":
            mode = nums[0] if nums else 0
            if mode == 2:
                self.cells = [[" "] * self.cols for _ in range(self.rows)]
            elif mode == 0:
                for k in range(self.c, self.cols):
                    self.cells[self.r][k] = " "
                for rr in range(self.r + 1, self.rows):
                    self.cells[rr] = [" "] * self.cols
        elif final == "A":
            self.r = max(0, self.r - arg(0, 1))
        elif final == "B":
            self.r = min(self.rows - 1, self.r + arg(0, 1))
        elif final == "C":
            self.c = min(self.cols - 1, self.c + arg(0, 1))
        elif final == "D":
            self.c = max(0, self.c - arg(0, 1))
        elif final in ("h", "l") and private and nums and nums[0] == 7:
            self.autowrap = final == "h"
        # m (couleurs/attributs) et le reste : sans effet sur la géométrie

    def line(self, row):
        return "".join(self.cells[row]).rstrip()

    def full_line(self, row):
        return "".join(self.cells[row])

    def text(self):
        return "\n".join(self.line(r) for r in range(self.rows))


class Session:
    def __init__(self, rows, cols, args=(), env_extra=None, term="xterm-256color"):
        self.screen = Screen(rows, cols)
        self.raw = ""
        env = dict(os.environ, TERM=term, LANG="C.UTF-8")
        env.pop("NO_COLOR", None)
        env.update(env_extra or {})
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.execve(BIN, [BIN, "--lang", "fr", *args], env)
        self.resize(rows, cols)
        self.buf = b""
        self.alive = True

    def resize(self, rows, cols):
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
        self.screen.resize(rows, cols)

    def _drain(self, timeout):
        end = time.time() + timeout
        got = False
        while time.time() < end:
            ready, _, _ = select.select([self.fd], [], [], 0.05)
            if not ready:
                if got:
                    break
                continue
            try:
                data = os.read(self.fd, 65536)
            except OSError:
                self.alive = False
                break
            if not data:
                self.alive = False
                break
            got = True
            self.buf += data
            try:
                text = self.buf.decode("utf-8")
                self.buf = b""
            except UnicodeDecodeError as e:
                text = self.buf[: e.start].decode("utf-8", "replace")
                self.buf = self.buf[e.start:]
            self.raw += text
            self.screen.feed(text)
        return got

    def prompts(self):
        """Nombre de prompts affichés (les codes couleur s'intercalent dans « ]\x1b[0m $ »)."""
        return ANSI.sub("", self.raw).count(PROMPT)

    def wait_prompt(self, count, timeout=20.0):
        end = time.time() + timeout
        while time.time() < end:
            self._drain(0.2)
            if self.prompts() >= count:
                # Le prompt est là : on laisse finir la redessinée des barres (machine chargée = lente).
                settle = time.time() + 2.0
                while time.time() < settle and self._drain(0.3):
                    pass
                return True
            if not self.alive:
                return False
        return False

    def send(self, line):
        os.write(self.fd, (line + "\n").encode())

    def finish(self, timeout=5.0):
        end = time.time() + timeout
        code = None
        while time.time() < end:
            self._drain(0.1)
            try:
                pid, status = os.waitpid(self.pid, os.WNOHANG)
            except ChildProcessError:
                break
            if pid:
                code = os.waitstatus_to_exitcode(status)
                break
        else:
            os.kill(self.pid, signal.SIGKILL)
            os.waitpid(self.pid, 0)
        self._drain(0.2)
        return code


passed = failed = 0


def check(name, ok, detail=""):
    global passed, failed
    if ok:
        passed += 1
        print(f"  ok    {name}")
    else:
        failed += 1
        print(f"  FAIL  {name}")
        if detail:
            print("        " + detail.replace("\n", "\n        "))


def play(rows, cols, commands, args=(), **kw):
    """Joue une liste de commandes (le nom du joueur en premier) et retourne la session."""
    s = Session(rows, cols, args, **kw)
    s._drain(0.6)
    s.send(commands[0])  # nom du hacker
    expected = 1
    for cmd in commands[1:]:
        s.wait_prompt(expected)
        s.send(cmd)
        expected += 1
    s.wait_prompt(expected)
    return s


def main():
    if not os.path.exists(BIN):
        print(f"exécutable introuvable : {BIN}", file=sys.stderr)
        return 2

    # --- Mise en page de base -------------------------------------------------
    s = play(30, 100, ["Neo", "scan"])
    scr = s.screen
    top, bottom = scr.line(0), scr.line(scr.rows - 1)
    check("barre du haut : nom, niveau, crédits, alerte",
          "Neo" in top and "Niv 1" in top and "100 ¢" in top and "ALERTE" in top, repr(top))
    check("barre du bas : commandes disponibles", "Commandes :" in bottom and "help" in bottom
          and "scan" in bottom and "bruteforce" not in bottom, repr(bottom))
    body = "\n".join(scr.line(r) for r in range(1, scr.rows - 1))
    check("la zone centrale contient le jeu (prompt, résultat du scan)",
          PROMPT in body and "localhost" in body)
    check("les deux barres occupent exactement toute la largeur",
          all(len(scr.full_line(r).replace("", "")) == 100 and scr.full_line(r)[-1] == " "
              for r in (0, scr.rows - 1)))
    check("l'alerte du scan (+1) est visible dans la barre", top.split("ALERTE")[-1].split()[-1] == "1", repr(top))

    # Beaucoup de sorties : le contenu défile, les barres restent en place.
    s.send("help"); s.wait_prompt(3)
    s.send("help"); s.wait_prompt(4)
    s.send("help"); s.wait_prompt(5)
    scr = s.screen
    check("après plusieurs écrans d'aide, la barre du haut est toujours en ligne 1",
          "Neo" in scr.line(0) and "ALERTE" in scr.line(0), repr(scr.line(0)))
    check("… et la barre du bas toujours en dernière ligne",
          "Commandes :" in scr.line(scr.rows - 1), repr(scr.line(scr.rows - 1)))
    check("… sans qu'aucune barre ne se retrouve au milieu de l'écran",
          not any("Commandes :" in scr.line(r) or "ALERTE" in scr.line(r) for r in range(1, scr.rows - 1)))
    check("le texte a bien défilé dans l'historique", len(scr.scrolled_off) > 20)

    # Le déblocage de bruteforce apparaît dans la barre.
    for _ in range(4):
        n = s.prompts()
        s.send("scan"); s.wait_prompt(n + 1)
    check("bruteforce apparaît en bas après le déblocage (niveau 2)",
          "bruteforce" in s.screen.line(s.screen.rows - 1) and "Niv 2" in s.screen.line(0),
          repr(s.screen.line(0)) + "\n" + repr(s.screen.line(s.screen.rows - 1)))

    # clear ne touche pas les barres.
    n = s.prompts()
    s.send("clear"); s.wait_prompt(n + 1)
    scr = s.screen
    check("clear : les barres sont conservées", "Neo" in scr.line(0)
          and "Commandes :" in scr.line(scr.rows - 1))
    body = "\n".join(scr.line(r) for r in range(1, scr.rows - 1))
    check("clear : la zone centrale est vidée (hors bandeau et prompt)", "localhost" not in body)
    check("clear : le bandeau est entièrement dans la zone centrale (rien sous la barre du haut)",
          "╔═" in body and "NEON HACK" in body and "╚═" in body)

    # Sortie propre : région rétablie, barres effacées.
    n = s.prompts()
    s.send("quit")
    code = s.finish()
    scr = s.screen
    check("quit : code de retour 0", code == 0, f"code={code}")
    check("quit : région de défilement rétablie", scr.region_resets >= 1 and scr.top == 0
          and scr.bottom == scr.rows - 1)
    check("quit : les deux barres sont effacées", "Neo |" not in scr.text() and "Commandes :" not in scr.text())
    check("quit : le message d'au revoir reste visible", "Merci d'avoir joué" in scr.text())

    # --- Langue --------------------------------------------------------------
    s = Session(30, 100, ["--lang", "en"])
    s._drain(0.6); s.send("Neo"); s.wait_prompt(1)
    check("--lang en : barres en anglais", "Lvl 1" in s.screen.line(0) and "ALERT" in s.screen.line(0)
          and "Commands:" in s.screen.line(s.screen.rows - 1), repr(s.screen.line(0)))
    s.send("quit"); s.finish()

    # --- Désactivations ---------------------------------------------------------
    s = play(30, 100, ["Neo", "status"], args=["--no-hud"])
    s.send("quit"); s.finish()
    check("--no-hud : aucune région de défilement, aucune barre",
          "\x1b[2;29r" not in s.raw and "Commandes :" not in s.raw)

    s = play(30, 100, ["Neo", "status"], env_extra={"TERM": "dumb"})
    s.send("quit"); s.finish()
    check("TERM=dumb : pas de HUD", "\x1b[2;29r" not in s.raw)

    s = play(20, 100, ["Neo", "status"])
    s.send("quit"); s.finish()
    check("terminal de 20 lignes : pas de HUD", "\x1b[2;19r" not in s.raw and "Commandes :" not in s.raw)
    s = play(30, 70, ["Neo", "status"])
    s.send("quit"); s.finish()
    check("terminal de 70 colonnes : pas de HUD", "Commandes :" not in s.raw)

    # --- Terminal étroit ---------------------------------------------------------
    s = play(24, 80, ["Neo", "scan"])
    top, bottom = s.screen.line(0), s.screen.line(s.screen.rows - 1)
    check("80x24 (minimum) : barres présentes et sans débordement",
          "Neo" in top and "ALERTE" in top and "Commandes :" in bottom
          and s.screen.r >= 1 and s.screen.r <= 22, repr(top) + "\n" + repr(bottom))
    s.send("quit"); s.finish()

    long_name = "Hacker" + "é" * 40 + "🧠" * 10
    s = play(30, 80, [long_name, "status"])
    top = s.screen.line(0)
    check("nom très long (accents, emoji) : la jauge d'alerte reste visible et rien ne déborde",
          "ALERTE" in top and top.count("\n") == 0, repr(top))
    s.send("quit"); s.finish()

    # --- Redimensionnement ---------------------------------------------------------
    s = play(30, 100, ["Neo", "status"])
    s.resize(40, 120)
    n = s.prompts()
    s.send("status"); s.wait_prompt(n + 1)
    scr = s.screen
    check("redimensionnement 40x120 : la barre du bas suit sur la nouvelle dernière ligne",
          "Commandes :" in scr.line(39), repr(scr.line(39)))
    s.resize(20, 100)  # trop petit : retour à l'affichage classique
    n = s.prompts()
    s.send("status"); s.wait_prompt(n + 1)
    check("redimensionnement sous 80x24 : le HUD se désactive proprement",
          s.screen.top == 0 and s.screen.bottom == s.screen.rows - 1)
    s.send("quit"); s.finish()

    # --- Ctrl+C : le terminal doit être rendu utilisable -----------------------
    s = play(30, 100, ["Neo", "status"])
    os.kill(s.pid, signal.SIGINT)
    s.finish()
    check("Ctrl+C : la région de défilement est rétablie", s.screen.region_resets >= 1
          and s.screen.top == 0 and s.screen.bottom == s.screen.rows - 1)

    # --- Fin d'entrée (Ctrl+D) ----------------------------------------------------
    s = play(30, 100, ["Neo", "status"])
    os.write(s.fd, b"\x04")
    code = s.finish()
    check("Ctrl+D : sortie propre, terminal rétabli", code == 0 and s.screen.region_resets >= 1
          and "Commandes :" not in s.screen.text(), f"code={code}")

    # --- --no-color : les barres restent lisibles sans couleur -----------------------
    s = play(30, 100, ["Neo", "status"], args=["--no-color"])
    check("--no-color : barres présentes, aucune séquence de couleur émise dans les barres",
          "Neo" in s.screen.line(0) and not re.search(r"\x1b\[(3[0-7]|9[0-7])m", s.raw.split(PROMPT, 1)[-1]),
          repr(s.screen.line(0)))
    s.send("quit"); s.finish()

    print(f"\nhud (pty) : {passed} réussi(s), {failed} échec(s)")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
