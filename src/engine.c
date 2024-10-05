#include "engine.h"
#include "libapi.h"

#include <stdio.h>
#include <unistd.h>

void run_engine(){
    int cycle = 1;

    while(cycle < 100){
        printf("Cycle: %i\n", cycle);
        cycle++;

        sleep(1);
    }
}