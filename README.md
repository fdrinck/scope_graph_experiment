## Scope graph experiment

Code experiment related to the concepts discussed
[here](https://pl.ewi.tudelft.nl/research/projects/scope-graphs/).

A toy language example is contained in `example.code`. Running `cargo run --
example.code` will show how the names have been resolved.

The import identifiers are not elaborated to explicit scopes but are resolved on
the fly by resolving the scope references one after another and then using the
resolved scope to continue resolution. Note that correctly the name scopes do
not have a parent scope link. This is for clarity and also to avoid the
unexpected situation that we could import symbols from parent scopes.

Because of the current heat wave I did not have the energy to write this code by
myself, nearly all of it is done by Gemini. My main interest was whether we can
find a cheap graph representation with very limited memory allocations. The code
is not highly optimized but it is already quite ok.
