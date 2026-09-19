#include "platform.h"

#include <stdio.h>

#ifdef _WIN32
#include <io.h>
#include <windows.h>
#else
#include <sys/ioctl.h>
#include <time.h>
#include <unistd.h>
#endif

static bool g_fast = false;

void nh_set_fast(bool fast) { g_fast = fast; }
bool nh_is_fast(void) { return g_fast; }

#ifdef _WIN32

#ifndef ENABLE_VIRTUAL_TERMINAL_PROCESSING
#define ENABLE_VIRTUAL_TERMINAL_PROCESSING 0x0004
#endif

void nh_platform_init(void)
{
    SetConsoleOutputCP(CP_UTF8);

    HANDLE out = GetStdHandle(STD_OUTPUT_HANDLE);
    DWORD mode = 0;
    if (out != INVALID_HANDLE_VALUE && GetConsoleMode(out, &mode))
    {
        SetConsoleMode(out, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
    }
}

void nh_sleep_ms(unsigned ms)
{
    if (g_fast || ms == 0)
        return;
    fflush(stdout);
    Sleep(ms);
}

bool nh_stdout_is_tty(void) { return _isatty(_fileno(stdout)) != 0; }

/* Non compilé ni testé sous Windows à ce jour (pas de chaîne de compilation croisée locale). */
bool nh_term_size(int *cols, int *rows)
{
    CONSOLE_SCREEN_BUFFER_INFO info;
    HANDLE out = GetStdHandle(STD_OUTPUT_HANDLE);
    if (out == INVALID_HANDLE_VALUE || !GetConsoleScreenBufferInfo(out, &info))
        return false;
    *cols = info.srWindow.Right - info.srWindow.Left + 1;
    *rows = info.srWindow.Bottom - info.srWindow.Top + 1;
    return *cols > 0 && *rows > 0;
}

#else

void nh_platform_init(void) {}

void nh_sleep_ms(unsigned ms)
{
    if (g_fast || ms == 0)
        return;
    fflush(stdout);

    struct timespec ts;
    ts.tv_sec = (time_t)(ms / 1000u);
    ts.tv_nsec = (long)(ms % 1000u) * 1000000L;
    while (nanosleep(&ts, &ts) == -1)
    {
        /* interrompu par un signal : reprendre le temps restant */
    }
}

bool nh_stdout_is_tty(void) { return isatty(STDOUT_FILENO) != 0; }

bool nh_term_size(int *cols, int *rows)
{
    struct winsize ws;
    if (ioctl(STDOUT_FILENO, TIOCGWINSZ, &ws) != 0 || ws.ws_col == 0 || ws.ws_row == 0)
        return false;
    *cols = ws.ws_col;
    *rows = ws.ws_row;
    return true;
}

#endif
