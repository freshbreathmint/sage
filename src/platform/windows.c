#include "platform/platform.h"

#if PLATFORM_WINDOWS

#include <stdlib.h>
#include <windows.h>

// Internal State (Windows)
typedef struct internal_state {
    HINSTANCE h_instance;
    HWND hwnd;
} internal_state;

// Forward declaration for window procedure callback
LRESULT CALLBACK window_proc(HWND hwnd, UINT msg, WPARAM w_param, LPARAM l_param);

void platform_init_window(platform_state *platform){
    // Allocate the internal state and cast to known type
    platform->internal = malloc(sizeof(internal_state));
    internal_state *state = (internal_state*)platform->internal;

    // handle to application instance
    state->h_instance = GetModuleHandle(NULL);

    // define window class
    HICON icon  = LoadIcon(state->h_instance, "APPLICATION_ICON");

    WNDCLASS wc = {0};
    wc.style = CS_DBLCLKS;  // enable double-clicking
    wc.lpfnWndProc = window_proc;
    wc.hInstance = state->h_instance;
    wc.hIcon = icon;
    wc.lpszClassName = "SageWindow";

    RegisterClass(&wc);

    // Window style
    int window_style;
    window_style |= WS_OVERLAPPED;  // Overlapped Window
    window_style |= WS_SYSMENU;     // System Menu
    window_style |= WS_CAPTION;     // Title Bar
    window_style |= WS_MAXIMIZEBOX; // Maximize Button
    window_style |= WS_MINIMIZEBOX; // Minimize Button
    window_style |= WS_THICKFRAME;  // Window Resize (Thick Frame)

    // create window
    HWND hwnd = CreateWindowEx(
        0,                              // Window Extended Style
        wc.lpszClassName,               // Class Name
        "Sage Engine",                  // Window Title
        window_style,                   // Window Style
        CW_USEDEFAULT, CW_USEDEFAULT,   // Window Position (x, y)
        500, 300,                       // Window Size (width, height)
        NULL,                           // Parent Window
        NULL,                           // Menu
        state->h_instance,              // Instance Handle
        NULL                            // lpParam
    );

    // Set the internal state
    state->hwnd = hwnd;

    //show window
    ShowWindow(state->hwnd, SW_SHOWNORMAL);
}

void platform_free_internal_state(platform_state *platform){
    internal_state *state = (internal_state*)platform->internal;

    // destroy window if it still exists
    if(state->hwnd){
        DestroyWindow(state->hwnd);
        state->hwnd = 0;
    }

    free(state);
}

void platform_process_message(){
    MSG message;
    while(PeekMessageA(&message, NULL, 0, 0, PM_REMOVE))
    {
        TranslateMessage(&message);
        DispatchMessage(&message);
    }
}

// window procedure function
LRESULT CALLBACK window_proc(HWND hwnd, UINT msg, WPARAM w_param, LPARAM l_param){
    switch(msg)
    {
        case WM_DESTROY:
            PostQuitMessage(0);
            return 0;
        default:
            // Pass unhandled messages to the default handler.
            return DefWindowProc(hwnd, msg, w_param, l_param);
    }
}

#endif //PLATFORM_WINDOWS