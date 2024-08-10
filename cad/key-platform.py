import cadquery as cq
from common import *

def plunger_to_screws(plunger_positions):
    screw_positions = []
    for p in plunger_positions:
        screw_positions.append((p[0], p[1] + SolenoidSupport.screw_hole_separation / 2))
        screw_positions.append((p[0], p[1] - SolenoidSupport.screw_hole_separation / 2))
    return screw_positions

def key_platform(num_white_keys):
    if num_white_keys == 1:
        plunger_positions = [(WhiteKey.width / 2, WhiteKey.plunger_hole_offset)]
    elif num_white_keys == 2:
        plunger_positions = [
            (WhiteKey.width * 1 / 2, WhiteKey.plunger_hole_offset),
            (WhiteKey.width        , BlackKey.plunger_hole_offset),
            (WhiteKey.width * 3 / 2, WhiteKey.plunger_hole_offset),
        ]
    elif num_white_keys == 7:
        plunger_positions = [
            (WhiteKey.width *  1 / 2, WhiteKey.plunger_hole_offset),
            (WhiteKey.width         , BlackKey.plunger_hole_offset),
            (WhiteKey.width *  3 / 2, WhiteKey.plunger_hole_offset),
            (WhiteKey.width *  2    , BlackKey.plunger_hole_offset),
            (WhiteKey.width *  5 / 2, WhiteKey.plunger_hole_offset),
            (WhiteKey.width *  7 / 2, WhiteKey.plunger_hole_offset),
            (WhiteKey.width *  4    , BlackKey.plunger_hole_offset),
            (WhiteKey.width *  9 / 2, WhiteKey.plunger_hole_offset),
            (WhiteKey.width *  5    , BlackKey.plunger_hole_offset),
            (WhiteKey.width * 11 / 2, WhiteKey.plunger_hole_offset),
            (WhiteKey.width *  6    , BlackKey.plunger_hole_offset),
            (WhiteKey.width * 13 / 2, WhiteKey.plunger_hole_offset),
        ]
    else:
        raise ValueError(f"Expected one of [1, 2, 7] for num_white_keys, got: {num_white_keys}")
    return (
        cq.Workplane("YZ")
        .vLine(KeyPlatform.bed_to_top)
        .hLine(KeyPlatform.length)
        .vLine(-KeyPlatform.thickness)
        .hLine(
            -KeyPlatform.length
            + KeyPlatform.chamfer
            + KeyPlatform.base_depth
        )
        .line(-KeyPlatform.chamfer, -KeyPlatform.chamfer)
        .vLine(
            -KeyPlatform.bed_to_top
            + KeyPlatform.thickness
            + KeyPlatform.chamfer
        )
        .close()
        .extrude(WhiteKey.width * num_white_keys)
        .faces(">Z")
        .workplane()
        .pushPoints(plunger_positions)
        .hole(Plunger.hole_diameter)
        .pushPoints(plunger_to_screws(plunger_positions))
        .hole(M3Screw.hole_diameter)
    )

export_stl(key_platform(1), "one-key-platform")
export_stl(key_platform(2), "three-key-platform")
export_stl(key_platform(7), "eleven-key-platform")
