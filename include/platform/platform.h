#pragma once

/* PLATFORM DETECTION */
#if defined(WIN32) || defined(_WIN32) || defined(__WIN32__)
    #define PLATFORM_WINDOWS 1
    #if defined(_WIN64)
        #define PLATFORM_WINDOWS_64 1
    #else
        #define PLATFORM_WINDOWS_32 1
    #endif
#elif defined(__linux__) || defined(__linux)
    #define PLATFORM_LINUX 1
#else
    #error "SAGE: Unknown/Unsupported Platform"
#endif

// Platform State
typedef struct platform_state {
    void* internal;
} platform_state;


void platform_init_window(platform_state *platform);
void platform_free_internal_state(platform_state *platform);
void platform_process_message();
