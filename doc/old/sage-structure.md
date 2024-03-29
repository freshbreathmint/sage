# Document Purpose

Stream of conciousness thoughts about Sage's structure, purpose, design, etc.

## Hot Reloadable vs Static

One thing that's confusing for me right now is deciding which parts of the engine should be dynamically hot-reloadable and what parts shouldn't.
On one hand, there's the ideal situation where almost all of the engine code is based on modules that can be enabled by the application on startup.

## Application Initialization

The application itself should be another library or set of libraries, that can be external to the Sage engine's folder, have Sage as a subfolder within it, etc.
The application should determine by itself all of the neccessary systems, resources, etc., to use, and it should provide this info to the engine when the program is started.

## Realization

One thing I just realized is that the deliniating line between what should be considered a system and what should be part of the base engine should be anything that the engine itself requires to run.

Some systems might require other systems as dependancies. For instance, if we have a logging system, that will need to be enabled for probably most other systems to work.
Should the logging system be integrated into the engine, for instance?

Bevy manages to do a similar thing to what I'm imagining with its crates, but the situation is different. I think that I still want Sage to be the executable, and for the application to be a library. Whereas, in Bevy, the application is the binary and it calls into bevy as a library to create things like the main loop etc., I want to have the main part of the Sage engine be this transformable game loop type thing.

Maybe for now a good policy might be to make only the application hot-reloadable and try to integrate everything into the main binary? I'm not really sure though. I feel like having the engine also be dynamically "built" at runtime might be useful.

## More About Logging Specifically

Another thing to think about logging: when to start an actual log? Kind of a moot point right now since we don't even have a logging system to copy to a file or anything, but it's obviously philisophically a good question, and it lead me to this thought:

What if we have a sort of 'boot' phase of the engine where everything is configured, the neccesary systems are enabled, the engine goes through it's pre-startup checks, and then we have an actual 'start' phase of the engine, which begins all these systems working in tandem together. Once the start phase of the engine is finished we move on to the boot phase of the application itself, and once the application is ready, we can run the engine and the application can start to interface with it.
