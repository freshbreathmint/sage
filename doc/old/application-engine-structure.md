# Document Purpose

Stream of conciousness ideas about the application-engine structure I want to make.

## Problem

Sage is a binary, which means that I need to develop my applications as a library that gets used by this binary dynamically at runtime.

It also needs to be built in a way that allows this application library or libraries to be dynamically loaded despite not being a dependancy. They should be able to be loaded from another project that may or may not have a copy of the Sage project inside of it.

The final structure of all of this should be friendly toward the easy creation of new projects. It should also support the application specifying what portions of the engine to use at runtime.

I guess one sort of way to visualize it would be like the engine being this closed system that the application sort of plugs into. Like a key in your car engine?

## Extra Note About Hot Reloading

Whatever the solution to this is should be tightly interwoven with the hot reloading system, so that there is basically a seamless hot reloading interface when optionally toggled.

The application should maybe communicate through to the engine with a small interface that requires the no-mangle tags or some other solution. But essentially I want almost all of the code of the application to be dynamically alterable at runtime. The engine should also be dynamically alterable, as well as any other plugin such as a potential vulkan library.

All of this should be able to be turned off dynamically when we are trying to make a stable release build. It should have no effect on release builds, with everything statically linked.

## Project Structure

The file structure should be kept as simple as possible. There might need to be some distinction between aspects of the engine that are hot reloadable and aspects that arent, such as (presuambly?) the hot reloading module. Although, I don't really know what the limits of this are? Maybe we can dynamically load hot-reloading? Who knows.

The decision to load hot loading would logically have to come first, in any start up sequence I think. Maybe some sort of build chain thing, where we are deciding what kind of build to make? There's a lot of interesting things to think about with this.
