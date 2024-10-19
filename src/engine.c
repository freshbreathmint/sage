#include "engine.h"
#include "libapi.h"

#include <stdlib.h>

void init_window(){
    engine_state state;
    state.platform = malloc(sizeof(platform_state)); //allocate memory for platform state

    platform_init_window(state.platform);

    for (int i = 0; i < 10000000; i++){
        platform_process_message();
    }

    platform_free_internal_state(state.platform);
    free(state.platform);
}