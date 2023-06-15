# WARNING!

This library is in active development, and as such I would not recommend using it for anything. Things may change very dramatically. Although this code is publically available, I'm not making it publically available because I think it's ready for public usage. I'm making it publically available so people can use it if they want to. There is some utility in using this library even if it's not complete.

# What is this?

This is a library that I'm developing as part of a collection of tools I'm developing related to Minecraft. Tools such as an NBT editor, a full world editor, tools for optimizing region files, and whatever else I can imagine. This library will work as a sort of backend into the inner workings of Minecraft data. This includes Minecraft's region file format, NBT format, SNBT format, and perhaps even tools for working with the Minecraft .jar file itself.

A few years ago, I was playing Minecraft and enjoying myself quite a lot, but I was irritated by how limited the creation tools felt. I even used some mods to make creating easier, but ultimately it wasn't what I wanted. I wanted a full scale editor. So I checked out MCEdit, which I thought was great, but it didn't run very well. It was written in Python, after all. I'm not trying to discredit the work of the creator, it was a monumental undertaking, but it needed to be much more optimized.

I'd already been programming for over a decade by the time I got the idea for this project, so it wasn't a matter of experience, it was a matter of research. So that's what I did. I think I spent maybe the first year doing various research, and even writing a small library in Python to work with Minecraft worlds. I never finished that library because I didn't want to waste any more time. I wanted to work on the world editor. But the problem was that I wanted to create the world editor with a language that was more bare metal, unlike Python, Java, or C#. So my choices were languages like: C, C++, or Rust. I'm sure there were other choices, but I really just saw it as two choices: Either C++, or Rust. I wanted to avoid the bloat of a game engine, so Unreal was out of the question for me. At first, I chose C++ for the project. I spent quite a lot of time writing thousands of lines of code for the NBT encoding and decoding portion of the library, then I updated my compiler and the whole project stopped building. So I decided to scrap it, and C++, and decided to learn Rust, which is something I had planned on doing for a while anyway. Boy, I was not ready for how much fun Rust was going to be. I'm so glad that I scrapped C++.

After writing thousands of (fun) lines of Rust, I've gotten to a point in the project where I hope that I'm ready to share what I've been working on. The ultimate plan is to have a solid codebase for working with Minecraft stuff.

# What can it do?

Here's a short list of some of the things I'm hoping people will be able to create with this library:
* World editors.
* World generators. (Maze generators, anyone?)
* NBT editors.
* World renderers.
* I dunno, whatever the heck you want!