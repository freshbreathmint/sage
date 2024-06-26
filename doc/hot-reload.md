# Hot Reload

Another stream of conciousness document focusing on the hot reload feature, trying to suss out why I need it.

## Doing It Myself

I've decided to write the hot reloader myself, for a couple reasons.

The existing hot reloader crate that I'm looking at has a lot of dependancies and is written in a not-exactly bespoke way.
I feel like I'll understand it's functions better at the very least if I try to recreate it myself but in a way that is written from the start to be compatable with the framework/engine.

## Path Forward

The path forward is to start from scratch trying to create a very basic but workable version of this hot reloading library.

During this process, the initial file structure of the project should be decided.

## Thoughts

### Fast Iterative Development Cycle

* Being able to hot reload the engine and application code on the fly would be immensely helpful for quickly iterating and understanding code.
* Almost neccessary if we don't want to build in large testing apparatus for immediately jumping to a game-state 'scenario', maintaining the application state during development can be important.

### It Doesn't Need To Be Perfect

* Hot Reloading doesn't need to be perfect, the goal should be that most underlying functions in the game code can be changed on the fly or rewritten between reloads.
* If, because of current limitations in Rust, it's infeasible to maintain the application state after changing a Type or it's implementation then that's probably fine.
* It's better to have the feature partially work than not at all.

### It Can't Hold The User Back

* One dealbreaker is definently that the user should not be limited as to what code they create inside of their application library.
* If using hot-reloading disables major features of the Rust compiler or the Rust language/syntax, it may not neccessarily be a good idea.

### Should Not Be Invasive

* Ideally, one shouldn't have to interact with the hot reloading system code outside of the main executable.
* If I have to attach no-mangle tags to literally everything in my application and engine systems, that would not be very fun or efficient.
* I think the risk regarding this can be circumnavigated by doing what I was already planning on doing and making everything modular. The only stuff that might have to be no-mangle tagged would be the application's ultimate 'update' functions etc that send data and commands to the framework/engine.

### Should Be Compatable With Multithreading

* I've heard there are problems with thread local storage and things like that when it comes to hot reloading the engine. I admit I don't really understand what this means because I don't have a good understanding of multithreading in the first place.
* But the point is, hot reloading code should be a tool in the toolbox that might not fit every use. If the application state is sometimes is unsalvagable or the code doesn't properly reload after changing a certain system, that's acceptable.
* But the sword swings both ways, if several hot reloads cause some sort of buildup of memory leaks that make developing untenable, that's not acceptable either.
* Ideally, everything should be built in a way that minimizes the amount of times that the application state is lost or that memory is leaked when doing a hot reload.
* It's not absolutely neccesary for every system but it is neccessary that it doesn't cause huge problems for EVERY hot reload even for basic things in the application itself.

### Priority One: Application

* Making the application work seamlessly with hot reloading is the #1 priority of this system.
* You should be able to change pretty much anything (within the reason of the system, obviously stuff like changing enums/struct implementations may not properly work, which is somewhat acceptable), without worrying that it is going to cause some sort of buildup of memory leaks or other cumulative issue with multiple hot reloads.
* This should also work for developing parts of the engine library, or any other library for that matter, for my own personal sanity.

### Build System

* I need a custom build system that allows me to run a hot reloadable version of the engine and a non hot reloadable version of the engine.
* Also maybe a release version.
* The point is that it should be flexible and allow me to run any of the compilable versions of the framework without hassling with the command line.
* Ideally I should be able to press a button in VS Code that says "run hot-reloadable" or something.
* If we are running the non-hot reloadable version, no code relating to the hot reloading system should run.
* The result should be fundimentally the same though, just without unncessary checking to reload the libraries.
