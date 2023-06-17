# WARNING!

This library is in active development, and as such I would not recommend using it for anything. Things may change very dramatically. Although this code is publically available, I'm not making it publically available because I think it's ready for public usage. I'm making it publically available so people can use it if they want to. There is some utility in using this library even if it's not complete.

# What is this?

This is a library that acts as a bridge between Rust and Minecraft data. So this library will allow you to load, edit, and save NBT data, it will even allow you to edit Minecraft worlds. I would write more, but I haven't finished the library.

# Current Capabilities:

* NBT: Load, Edit, Save
* Region Files: Load, Save, Optimize (not finished).
* SNBT (JSON-like text-based NBT format) read/write

# Plans:

* Ability to open, edit, and save entire Minecraft worlds (past some specific version, otherwise having a converter to some internal format)
* Tools based on this library, such as a full world editor that hopefully will have its own list of features and plans eventually.
* NBT Editor: A tool that can be used to edit NBT files, or otherwise edit the extracted NBT from Region files.

# What can it do?

Here's a short list of some of the things I'm hoping people will be able to create with this library:
* World editors.
* World generators. (Maze generators, anyone?)
* NBT editors.
* World renderers.
* I dunno, whatever the heck you want!

# Collaboration?

Currently, I am not looking for collaborators. This is a personal project that I'm doing for fun. You are welcome to fork this repo and add onto it however you please (in accordance with the attached license), but I do not wish to have anyone else mucking around in my codebase at this time. Perhaps at a later point when the project is finished I may open it up for collaboration to allow people to make my code better, but this is a project that I'm soloing out of some masochistic desire within myself.