from lib.utils import get_dir_location, to_camel_case
from lib.code.utils import clean_property_name
from .blocks import get_property_struct_name
from ..mappings import Mappings

COLLISION_BLOCKS_RS_DIR = get_dir_location(
    '../azalea-physics/src/collision/blocks.rs')


def generate_block_shapes(blocks: dict, shapes: dict, aabbs: dict, block_states_report, block_datas_burger, mappings: Mappings):
    blocks, shapes = simplify_shapes(blocks, shapes, aabbs)

    code = generate_block_shapes_code(
        blocks, shapes, block_states_report, block_datas_burger, mappings)
    with open(COLLISION_BLOCKS_RS_DIR, 'w') as f:
        f.write(code)


def simplify_shapes(blocks: dict, shapes: dict, aabbs: dict):
    new_id_increment = 0

    new_shapes = {}
    old_id_to_new_id = {}

    old_id_to_new_id[None] = 0
    new_shapes[0] = ()
    new_id_increment += 1

    used_shape_ids = set()
    # determine the used shape ids
    for block_id, block_data in blocks.items():
        block_id = block_id.split(':')[-1]
        block_shapes = [state.get('collision_shape')
                        for state in block_data['states'].values()]
        for s in block_shapes:
            used_shape_ids.add(s)

    for shape_id, shape in enumerate(shapes):
        if shape_id not in used_shape_ids: continue
        # pixlyzer gives us shapes as an index or list of indexes into the
        # aabbs list
        # and aabbs look like { "from": number or [x, y, z], "to": (number or vec3) }
        # convert them to [x1, y1, z1, x2, y2, z2]
        shape = [shape] if isinstance(shape, int) else shape
        shape = [aabbs[shape_aabb] for shape_aabb in shape]
        shape = tuple([(
            (tuple(part['from']) if isinstance(
                part['from'], list) else ((part['from'],)*3))
            + (tuple(part['to']) if isinstance(part['to'], list)
               else ((part['to'],)*3))
        ) for part in shape])

        old_id_to_new_id[shape_id] = new_id_increment
        new_shapes[new_id_increment] = shape
        new_id_increment += 1

    # now map the blocks to the new shape ids
    new_blocks = {}
    for block_id, block_data in blocks.items():
        block_id = block_id.split(':')[-1]
        block_shapes = [state.get('collision_shape')
                        for state in block_data['states'].values()]
        new_blocks[block_id] = [old_id_to_new_id[shape_id]
                                for shape_id in block_shapes]

    return new_blocks, new_shapes


def generate_block_shapes_code(blocks: dict, shapes: dict, block_states_report, block_datas_burger, mappings: Mappings):
    # look at downloads/generator-mod-*/blockCollisionShapes.json for format of blocks and shapes

    generated_shape_code = ''
    for (shape_id, shape) in sorted(shapes.items(), key=lambda shape: int(shape[0])):
        generated_shape_code += generate_code_for_shape(shape_id, shape)

    # BlockState::PurpurStairs_NorthTopStraightTrue => &SHAPE24,
    generated_match_inner_code = ''
    shape_ids_to_variants = {}
    for block_id, shape_ids in blocks.items():
        if isinstance(shape_ids, int):
            shape_ids = [shape_ids]
        block_report_data = block_states_report['minecraft:' + block_id]
        block_data_burger = block_datas_burger[block_id]

        for possible_state, shape_id in zip(block_report_data['states'], shape_ids):
            variant_values = []
            for value in tuple(possible_state.get('properties', {}).values()):
                variant_values.append(to_camel_case(value))

            if variant_values == []:
                variant_name = to_camel_case(block_id)
            else:
                variant_name = f'{to_camel_case(block_id)}_{"".join(variant_values)}'

            if shape_id not in shape_ids_to_variants:
                shape_ids_to_variants[shape_id] = []
            shape_ids_to_variants[shape_id].append(
                f'BlockState::{variant_name}')
    # shape 1 is the most common so we have a _ => &SHAPE1 at the end
    del shape_ids_to_variants[1]
    for shape_id, variants in shape_ids_to_variants.items():
        generated_match_inner_code += f'{"|".join(variants)} => &SHAPE{shape_id},\n'

    return f'''
//! Autogenerated block collisions for every block

// This file is generated from codegen/lib/code/block_shapes.py. If you want to
// modify it, change that file.

#![allow(clippy::explicit_auto_deref)]
#![allow(clippy::redundant_closure)]

use super::VoxelShape;
use crate::collision::{{self, Shapes}};
use azalea_block::*;
use once_cell::sync::Lazy;

pub trait BlockWithShape {{
    fn shape(&self) -> &'static VoxelShape;
}}

{generated_shape_code}

impl BlockWithShape for BlockState {{
    fn shape(&self) -> &'static VoxelShape {{
        match self {{
            {generated_match_inner_code}_ => &SHAPE1
        }}
    }}
}}
'''


def generate_code_for_shape(shape_id: str, parts: list[list[float]]):
    def make_arguments(part: list[float]):
        return ', '.join(map(lambda n: str(n).rstrip('0'), part))
    code = ''
    code += f'static SHAPE{shape_id}: Lazy<VoxelShape> = Lazy::new(|| {{'
    steps = []
    if parts == ():
        steps.append('collision::empty_shape()')
    else:
        steps.append(f'collision::box_shape({make_arguments(parts[0])})')
        for part in parts[1:]:
            steps.append(
                f'Shapes::or(s, collision::box_shape({make_arguments(part)}))')

    if len(steps) == 1:
        code += steps[0]
    else:
        code += '{\n'
        for step in steps[:-1]:
            code += f'    let s = {step};\n'
        code += f'    {steps[-1]}\n'
        code += '}\n'
    code += '});\n'
    return code
