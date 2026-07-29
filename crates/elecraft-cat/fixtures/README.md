# Elecraft CAT transcript fixtures

These fixtures are cross-platform test data for the M1 CAT core. Each record is:

```text
delay_ms|command|response
```

A blank response means the SET has no immediate response. `~` splits one response into
incremental chunks. `!timeout` ends the exchange without a response, while
`!timeout:partial~chunks` supplies bytes received before expiry.

Required metadata:

```text
# provenance=document-derived
# profile=kx2
# source=https://example.invalid/primary-source
```

Future human captures must instead use `provenance=human-captured` and add a stable
`capture-id` tied to the issue #13 evidence record. The parser rejects an unlabeled
human capture. Document-derived and human-captured fixtures must remain in separate
directories.

## Included provenance

All sources were accessed 2026-07-29.

| Fixture | Evidence class | Primary source | What is derived |
| --- | --- | --- | --- |
| `document-derived/kx2.cat` | Document-derived synthetic; not a physical result | [Elecraft K3S/K3/KX3/KX2 Programmer's Reference, Rev G5](https://ftp.elecraft.com/KX2/Manuals%20Downloads/K3S%26K3%26KX3%26KX2%20Pgmrs%20Ref%2C%20G5.pdf) | KX2 product code, fixed command/response fields, example revision shape, 5/100/500 ms timing cases |
| `document-derived/kx3.cat` | Document-derived synthetic; not a physical result | [Elecraft K3S/K3/KX3/KX2 Programmer's Reference, Rev G5](https://ftp.elecraft.com/KX2/Manuals%20Downloads/K3S%26K3%26KX3%26KX2%20Pgmrs%20Ref%2C%20G5.pdf) | KX3 product code/options example, fixed command/response fields, example revision shape, 5/100/500 ms timing cases |

The frequencies, option combinations, and revision values exercise documented syntax;
they do not claim the state or firmware of a real KX2 or KX3. There are no
human-captured radio transcripts in this repository.
