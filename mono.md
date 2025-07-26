# Mono Todo

In case we want to make it work for Unity games:

- Inject into the process and get the Mono instance, look at:
    - https://github.com/wledfor2/MonoJunkie/blob/master/MonoJunkie/InjectionInternals.cpp#L133
    - https://github.com/wledfor2/MonoJunkie/blob/master/MonoJunkie/MonoJunkie.cpp#L54
    - https://github.com/TTENSHII/XashInjector/blob/dev/src/injector/MonoModule.cpp#L68
    - https://github.com/mizt0/mono-inject-rs/blob/main/src/main.rs#L152 (The dll to reference is either `mono.dll` for
      older games, or `mono-2.0-bdwgc.dll` for newer games)
- Either then invoke static methods through rust (assuming a static Player instance is available) and save everything.
- Or load a specialised C# plugin which can directly reference managed code and get the required data.