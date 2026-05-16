# Abrase

<img src="./abrase.png" alt="Abrase" width="100" style="float: right; margin-left: 20px;">

Abrase (`.abe`, abbreviated **Abe**) is a Rust dialect optimized for language models to make use of long context windows. Abrase source compiles to **Polka** bytecode, which runs on the **Myriad** runtime.

Compiler type & behavior checks are made explicit to hinder hallucination and help local inference. It features:

* static type check
* effect system
* simplified lifecycle management

It can be added to any Rust application. See [wiki](https://github.com/KHN190/Abrase/wiki).
