//! A simple program to generate SVGs for all schedules.
//!
//! Very useful for debugging and understanding system ordering.
//!
//! See the `system-graph/graphs/Main/svg` folder for the generated SVGs.

use std::path::PathBuf;

use azalea::{
    app::App,
    ecs::{schedule::Schedules, world::World},
    swarm::DefaultSwarmPlugins,
    DefaultBotPlugins, DefaultPlugins,
};
use bevy_mod_debugdump::schedule_graph::{
    self,
    settings::{Settings, Style},
};

fn main() -> Result<(), std::io::Error> {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, DefaultBotPlugins, DefaultSwarmPlugins));
    app.finish();

    graph_world("Main", app.world_mut())?;

    Ok(())
}

/// Graph all schedules in a [`World`].
fn graph_world(name: &str, world: &mut World) -> Result<(), std::io::Error> {
    let current = PathBuf::from(file!());
    let src_folder = current.parent().unwrap().parent().unwrap();

    // Get or create dot and svg folders.
    let dot_folder = src_folder.join("graphs").join(name).join("dot");
    if !dot_folder.exists() {
        std::fs::create_dir_all(&dot_folder)?;
    }

    let svg_folder = src_folder.join("graphs").join(name).join("svg");
    if !svg_folder.exists() {
        std::fs::create_dir_all(&svg_folder)?;
    }

    // Generate a graph for all schedules.
    world.resource_scope::<Schedules, _>(|world, schedules| {
        for (label, schedule) in schedules.iter() {
            // Write the dot file.
            let dot_path = dot_folder.join(format!("{label:?}.dot"));
            std::fs::write(
                &dot_path,
                schedule_graph::schedule_graph_dot(schedule, world, &settings()),
            )?;

            // Use `dot` to convert to svg.
            let svg_path = svg_folder.join(format!("{label:?}.svg"));
            std::process::Command::new("dot")
                .arg("-Tsvg")
                .arg(dot_path)
                .arg("-o")
                .arg(svg_path)
                .output()?;
        }

        Ok(())
    })
}

/// Returns the [`Settings`] for creating graphs.
fn settings() -> Settings {
    Settings {
        style: Style::dark_github(),
        ..Default::default()
    }
}
