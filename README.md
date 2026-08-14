## Scope graph experiment

Code experiment related to the concepts discussed
[here](https://pl.ewi.tudelft.nl/research/projects/scope-graphs/).

The toy language example is contained in `example.code`. All names are resolved
except for `offset` which has no suitable definition to resolve to.

The import identifiers are not elaborated to explicit scopes but are resolved on
the fly by resolving the scope references one after another and then using the
resolved scope to continue resolution. Note that correctly the name scopes do
not have a parent scope link. This is for clarity and also to avoid the
unexpected situation that we could import symbols from parent scopes.

Because of the current heat wave I did not have the energy to write this code by
myself, mostly. About 95% are done by Gemini. My main interest was whether we
can find a cheap graph representation with very limited memory allocations. The
code is not highly optimized but it is already quite ok.
