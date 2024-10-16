#include "engine.h"
#include "libapi.h"

#include "platform/platform.h"

void init_window(){
    platform_init_window();

    while(1){
        platform_process_message();
    }
}