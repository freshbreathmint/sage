# Purpose

Right now this document is just going to be stream of conciousness thoughts about what I'm trying to make and do.

---

## Thoughts

### Sage should be modular

* Meaning the application should have to explicitly initialize every needed feature of the engine itself. The application should have to specify that the program has a window, that it requires a render graph, that it requires input, etc.
* This modular structure allows for the creation of technically anything, but the core of the engine should be that it is some sort of application that requires a main loop and multiple phases of execution. The application can then specify that it wants the engine to enable certain systems, which it can then call upon.
  
### Sage should have a Hot Reloading friendly structure

* Meaning the structure of the program should be in a way that we are able to easily modify modules of the engine or the application, modify assets, etc. while still in runtime, in order to preserve the state of the program while developing, to ease development.
* This is difficult, because it seems like Rust doesn't have very good support for dynamic libraries.

### Sage should be a general framework

* Meaning:  Like a general library on steroids. It should do anything and everything under the hood as some sort of big bootstrapped thing. The application becomes another module with the relevant logic that plugs into the engine and provides it with the sort of glue to keep the systems working together.
* Kind of a stupid goal but it should strive to do everything generally. I don't want it to be too verbose/explicit to the point where everything is like an endless chain of submodules, but I do want things to be handled in a way that allows for the creation of generally anything I want.
* I dont really want to do anything other than make a very simple video game right now, so that's what I'll use as sort of a guiding principal to drive the development of this general framework.

### Do it yourself

* This is kind of another guiding principal. I should try not to use too many external crates without knowing exactly what they are accomplishing. If they can be accomplished similarly with very little work and in a more bespoke manner, then do it yourself.
* If it's just some simple feature that requires a lot of edge cases or something, then use a crate or whatever. I don't know, it's Rust.

### Don't give up

* Self explanitory.
