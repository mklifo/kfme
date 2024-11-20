[![checks](https://github.com/mklifo/kfme/actions/workflows/checks.yml/badge.svg)](https://github.com/mklifo/kfme/actions/workflows/checks.yml)
[![deploy](https://github.com/mklifo/kfme/actions/workflows/deploy.yml/badge.svg)](https://github.com/mklifo/kfme/actions/workflows/deploy.yml)

# `kfme`

Editing tool for keyframe motion files.

## Usage

```
Usage: kfme <COMMAND>

Commands:
  patch    Applies a patch to the given source file
  convert  Converts the format of a given source file
  build    Builds a binary and a corresponding header file from the given source file
  help     Print this message or the help of the given subcommand(s)
```

## Patch Files

Patch files are structured as a series of actions that are evaluated in order. They allow precise modifications to animations, transitions, and other components of a keyframe motion file.

### Adding Animations

To add a new animation, specify its attributes in an `add` action.

```yaml
anims:
- add:
    id: 20
    path: path/to/file.kf
    index: 0
    trans:
    - id: /.*/
      type: default_non_sync
```

### Deleting Animations

To delete an animation, specify its `id` in a `delete` action.

```yaml
anims:
- delete:
    id: 19
```

### Updating Animations

To modify an existing animation, use an `update` action and specify only its updated attributes.

```yaml
anims:
- update:
    id: 10
    index: 1
    trans:
    - delete:
        id: /.*/
    - add:
        id: 9
        type: chain_animation
```

### Nested Actions

Actions can be nested by attribute or field to perform complex operations. For instance:

```yaml
anims:
- update:
    id: 10
    trans:
    - delete:
        id: /.*/
    - add:
        id: 9
        type: chain_animation
```

### Combining Actions

Patch files can include multiple actions in sequence. For example:

```yaml
anims:
- add:
    id: 20
    path: path/to/file.kf
    index: 0
    trans:
    - id: /.*/
      type: default_non_sync
- delete:
    id: 19
```