from cadquery import exporters
import cadquery as cq
import argparse
import os

from ocp_vscode import show_object, set_defaults, Camera

# All dimensions are in millimeters
# Vertical datum is the plane of the white keys


class WhiteKey:
    width = 23.6
    length = 152.4
    elevation_up = 0
    elevation_down = -10.5
    travel = elevation_up - elevation_down


class BlackKey:
    width = 10.2
    length = 101.6
    elevation_up = 11.9
    elevation_down = 3.6
    travel = elevation_up - elevation_down


class SiliconeFeet:
    thickness = 3.8
    diameter = 12


# See https://en.wikipedia.org/wiki/Lumber#North_American_softwoods
class DimensionalLumber:
    one_inch = 19  #      3/4"
    four_inches = 89  # 3-1/2"


class Plunger:
    diameter = 9.525  # 3/8"
    height = 42
    hole_diameter = diameter + 2


class Steel:
    thickness = 2


class Solenoid:
    height = 32
    outer_diameter = Plunger.hole_diameter + 11.5


class M3Screw:
    diameter = 3
    hole_diameter = 3.5
    hole_buffer_diameter = 10


class SolenoidSupport:
    wall_thickness = 0.6
    base_thickness = 1.8
    inner_diameter = Plunger.diameter + 0.8
    taper_fraction = 0.15
    taper_size = 5
    screw_hole_separation = 42.225


class KeySupport:
    clearance = 10
    white_plunger_hole_pos = 26
    black_plunger_hole_pos = 30
    thickness = 3
    elevation = BlackKey.elevation_up + clearance + thickness


class PlungerExtension:
    head_diameter = 20.3
    head_thickness = 2
    head_fuzz_thickness = 4
    # set the stem lengths such that the plungers are halfway out in the down position, which seems
    # to give the greatest efficiency for holding
    white_key_stem_length = (
        KeySupport.elevation
        + SolenoidSupport.base_thickness
        + Solenoid.height
        - WhiteKey.elevation_down
        - Plunger.height / 2
        - head_thickness
        - head_fuzz_thickness
    )
    black_key_stem_length = (
        KeySupport.elevation
        + SolenoidSupport.base_thickness
        + Solenoid.height
        - BlackKey.elevation_down
        - Plunger.height / 2
        - head_thickness
        - head_fuzz_thickness
    )


class Support:
    wall_thickness = KeySupport.thickness
    inner_gap = 100
    inner_gap_chamfer = 10
    wood_tol = 0.5
    wood_gap_width = DimensionalLumber.one_inch + wood_tol * 2
    wood_gap_height = DimensionalLumber.four_inches + wood_tol * 2
    depth = wall_thickness * 4 + inner_gap + wood_gap_width * 2
    height = wall_thickness * 2 + DimensionalLumber.four_inches + wood_tol * 2
    elevation = KeySupport.elevation - wall_thickness
    platform_elevation = 6.25
    spacer_height = elevation - platform_elevation - SiliconeFeet.thickness
    spacer_wall_thickness = 8

    @classmethod
    def make_profile(cls, width, chamfer=False):
        support = cq.Workplane("YZ")
        support = support.box(Support.depth, Support.height, width, centered=False)
        support = (
            support.moveTo(Support.wall_thickness, Support.wall_thickness)
            .rect(Support.wood_gap_width, Support.wood_gap_height, centered=False)
            .moveTo(Support.depth - Support.wall_thickness, Support.wall_thickness)
            .rect(-Support.wood_gap_width, Support.wood_gap_height, centered=False)
        )
        support = support.moveTo(
            Support.wood_gap_width + Support.wall_thickness * 2, Support.wall_thickness
        ).rect(Support.inner_gap, Support.wood_gap_height, centered=False)
        support = support.cutThruAll()
        if chamfer:
            support = support.edges("|X")[9, 11].chamfer(Support.inner_gap_chamfer)
        return support


class ControllerPCB:
    length = 127.6
    width = 40.9275
    thickness = 1.57


class Nema17Motor:
    body_width = 42.3
    body_height = 42.3
    body_length = 34
    lip_diameter = 22
    shaft_diameter = 5
    shaft_flat_diameter = 4.5
    shaft_length = 24
    mounting_hole_positions = [
        (-15.5, -15.5),
        (-15.5, 15.5),
        (15.5, 15.5),
        (15.5, -15.5),
    ]


def export_stl(model, name):
    pwd = os.path.dirname(os.path.abspath(__file__))
    exporters.export(model, f"{pwd}/{name}.stl")


def run(build, export):
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "-e", "--export", action="store_true", help="Export model as STL"
    )
    args = parser.parse_args()
    if args.export:
        export()
    else:
        set_defaults(reset_camera=Camera.KEEP)
        show_object(build())
