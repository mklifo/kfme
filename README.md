[![checks](https://github.com/mklifo/kfme/actions/workflows/checks.yml/badge.svg)](https://github.com/mklifo/kfme/actions/workflows/checks.yml)

# `kfme`

Editing tool for keyframe motion files.

## Usage

```
kfme <COMMAND>

Commands:
  convert  Converts a source file's format (inferred from file extensions)
  patch    Applies a patch to a given source file
  help     Print this message or the help of the given subcommand(s)
```

## Patch Files

Patch files are YAML-based, structured as a sequence of imperative actions that modify a source file. Each action can either add, update, or delete elements within the file.

### Example Patch

```yaml
anims:
- add:
    id: 0
    path: foo/bar.kfm
    index: 0
    trans:
    - id: 1
      type: blend
- update:
    id: 1
    index: 2
    trans:
    - delete:
        id: /.*/
    - add:
        id: 3
        type: chain_animation
```
