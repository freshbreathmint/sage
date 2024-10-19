#pragma once

#include "platform/platform.h"

typedef struct engine_state {
    platform_state *platform;
} engine_state;

void init_window();