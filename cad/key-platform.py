import argparse

from enum import StrEnum
from typing import Iterable

import cadquery as cq

from ocp_vscode import show_object, set_defaults, Camera

from common import (
    WhiteKey,
    BlackKey,
    KeyPlatform,
    Plunger,
    PlatformRib,
    M3Screw,
    SolenoidSupport,
    export_stl,
    self_tapping_hole,
    PCBSupport,
)

"""
TODO
- PCB mounting holes
- check vertical positioning (rubber feet?)
"""


def plunger_to_screws(
    plunger_positions: list[tuple[float, float]],
) -> list[tuple[float, float]]:
    screw_positions = []
    for p in plunger_positions:
        screw_positions.append((p[0], p[1] + SolenoidSupport.screw_hole_separation / 2))
        screw_positions.append((p[0], p[1] - SolenoidSupport.screw_hole_separation / 2))
    return screw_positions


class Key(StrEnum):
    W_LEFT = "L"
    W_MIDDLE = "M"
    W_RIGHT = "R"
    W_FULL = "X"
    B_LEFT = "l"
    B_MIDDLE = "m"
    B_RIGHT = "r"


def make_key_platform(pattern: Iterable[Key]) -> cq.Solid:
    platform = cq.Workplane().tag("platform")
    num_white_keys = len(
        list(
            filter(
                lambda k: k in [Key.W_LEFT, Key.W_MIDDLE, Key.W_RIGHT, Key.W_FULL],
                pattern,
            )
        )
    )
    base_width = WhiteKey.width * num_white_keys
    start_of_black_key = WhiteKey.length - BlackKey.length
    adjusted_black_key_width = BlackKey.width * 2
    plunger_hole_positions = []
    screw_hole_positions = []
    rib_positions = []
    pos = 0
    previous_key = None
    w_idx = 0
    for idx, key in enumerate(pattern):
        platform = platform.workplaneFromTagged("platform")
        platform = platform.center(pos, 0)
        match key:
            case Key.W_LEFT:
                platform = platform.vLine(WhiteKey.length)
                platform = platform.hLine(WhiteKey.width - adjusted_black_key_width / 2)
                platform = platform.vLine(-BlackKey.length)
                platform = platform.hLine(adjusted_black_key_width / 2)
                platform = platform.vLine(-start_of_black_key)
            case Key.W_MIDDLE:
                platform = platform.vLine(start_of_black_key)
                platform = platform.hLine(adjusted_black_key_width / 2)
                platform = platform.vLine(BlackKey.length)
                platform = platform.hLine(WhiteKey.width - adjusted_black_key_width)
                platform = platform.vLine(-BlackKey.length)
                platform = platform.hLine(adjusted_black_key_width / 2)
                platform = platform.vLine(-start_of_black_key)
            case Key.W_RIGHT:
                platform = platform.vLine(start_of_black_key)
                platform = platform.hLine(adjusted_black_key_width / 2)
                platform = platform.vLine(BlackKey.length)
                platform = platform.hLine(WhiteKey.width - adjusted_black_key_width / 2)
                platform = platform.vLine(-WhiteKey.length)
            case Key.W_FULL:
                platform = platform.vLine(WhiteKey.length)
                platform = platform.hLine(WhiteKey.width)
                platform = platform.vLine(-WhiteKey.length)
            case Key.B_LEFT | Key.B_MIDDLE | Key.B_RIGHT:
                platform = platform.move(0, start_of_black_key)
                platform = platform.hLine(adjusted_black_key_width / 2)
                platform = platform.vLine(BlackKey.length)
                platform = platform.hLine(-adjusted_black_key_width)
                platform = platform.vLine(-BlackKey.length)
            case _:
                err = f"Invalid key value '{key}'"
                raise ValueError(err)
        platform = platform.close()
        platform = platform.extrude(KeyPlatform.thickness)
        if key in [Key.W_LEFT, Key.W_FULL] and previous_key == Key.W_RIGHT:
            rib_positions.append((-pos, 0))
        match key:
            case Key.W_LEFT | Key.W_MIDDLE | Key.W_RIGHT | Key.W_FULL:
                key_midpoint = pos + WhiteKey.width / 2
                plunger_hole_positions.append(
                    (key_midpoint, KeyPlatform.white_plunger_hole_pos)
                )
                if w_idx in [1, 5]:
                    screw_hole_positions += [
                        (key_midpoint, PCBSupport.inner_y),
                        (key_midpoint, PCBSupport.outer_y),
                    ]
                w_idx += 1
                if idx < len(pattern) - 1:
                    pos += WhiteKey.width
            case Key.B_LEFT | Key.B_MIDDLE | Key.B_RIGHT:
                plunger_hole_positions.append(
                    (
                        pos,
                        WhiteKey.length
                        - BlackKey.length
                        + KeyPlatform.black_plunger_hole_pos,
                    )
                )
            case _:
                err = f"Invalid key value '{key}'"
                raise ValueError(err)
        previous_key = key

    # plunger holes
    platform = platform.faces(">Z")
    platform = platform.vertices("<XY")
    platform = platform.workplane(centerOption="CenterOfMass")
    platform = platform.pushPoints(plunger_hole_positions)
    platform = platform.hole(Plunger.hole_diameter)

    # screw holes
    screw_hole_positions += plunger_to_screws(plunger_hole_positions)
    platform = platform.pushPoints(screw_hole_positions)
    platform = platform.hole(M3Screw.hole_diameter)

    # base
    platform = platform.faces("<Y")
    platform = platform.workplane()
    platform = platform.transformed(rotate=cq.Vector(0, 0, -90))
    platform = platform.box(
        KeyPlatform.height,
        base_width,
        KeyPlatform.base_depth,
        centered=False,
    )

    # base window
    platform = platform.faces("<Y")
    platform = platform.workplane()
    platform = platform.transformed(rotate=cq.Vector(0, 0, 180))
    platform = platform.moveTo(
        -base_width / 2, KeyPlatform.thickness + PlatformRib.thickness
    )
    platform = platform.rect(
        base_width * KeyPlatform.window_fraction,
        KeyPlatform.height - PlatformRib.thickness,
        centered=(True, False),
    )
    platform = platform.cutThruAll()

    # extend platform to full length
    platform = platform.faces(">Y")
    platform = platform.wires()
    platform = platform.toPending()
    platform = platform.extrude(-(KeyPlatform.length - WhiteKey.length))

    # ribs
    platform = platform.faces("-Z")
    platform = platform.faces(">Z")
    platform = platform.workplane()
    platform = platform.transformed(rotate=cq.Vector(0, 0, 180))
    platform = platform.center(0, KeyPlatform.base_depth)
    platform = platform.pushPoints(rib_positions)
    platform = platform.box(
        PlatformRib.width,
        PlatformRib.length,
        PlatformRib.thickness,
        centered=(True, False, False),
    )

    # screw holes for joining to adjacent modules
    top = platform.faces(">Z")
    top = top.workplane()
    top = top.transformed(offset=cq.Vector(0, -KeyPlatform.base_depth))
    top = top.pushPoints(
        [(5, KeyPlatform.base_depth / 2), (base_width - 5, KeyPlatform.base_depth / 2)]
    )
    holes = top.eachpoint(
        lambda loc: self_tapping_hole(loc, M3Screw.hole_diameter / 2, 15)
    )
    platform = platform.cut(holes)

    return platform


module_patterns = [
    "LrRLlMrRLlM",
    "mMrRLlMrRLl",
    "MmMrRLlMrRL",
    "lMmMrRLlMrR",
    "LlMmMrRLlMr",
    "RLlMmMrRLlM",
    "rRLlMmMrRLl",
    "MrRLlMmMrRX",
]

parser = argparse.ArgumentParser()
parser.add_argument(
    "-e", "--export", action="store_true", help="Export all eight module models as STL"
)
args = parser.parse_args()

if args.export:
    for idx, pattern in enumerate(module_patterns):
        export_stl(make_key_platform(pattern), f"key-platform-{idx}")
else:
    set_defaults(reset_camera=Camera.KEEP)
    show_object(make_key_platform(module_patterns[3]))
