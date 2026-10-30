# templates/event

The event repo template that `advent init` writes out. It is embedded into the `advent`
binary and also sealed and verified by the engine's integration tests.

`examples/` holds one sample day per puzzle type, in plaintext on purpose: they are
public examples, not secrets. The plaintext guard exempts this folder.
