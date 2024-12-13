# Azalea Graph

A crate that generates system graph SVGs for azalea's bevy schedules.

The ones most likely to be useful are `GameTick`, `PreUpdate`, `Update`, and `PostUpdate`.

> [!NOTE]
> Graphs are not shared, so you'll need to generate them yourself.

## Usage

All generated graphs are saved inside `system-graph/graphs`

```bash
# Enter the `system-graph` directory
cd system-graph

# Generate all graphs
cargo run
```
