from cadquery import exporters
import cadquery as cq
import os

# All dimensions are in millimeters


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


class KeyPlatform:
    clearance = 10
    thickness = 2
    base_depth = 12
    length = 171.5
    window_fraction = 0.65
    white_plunger_hole_pos = 26
    black_plunger_hole_pos = 30
    silicone_feet_thickness = 3.8


class PlatformRib:
    width = 3.6
    thickness = 4
    length = KeyPlatform.length - 15


class WhiteKey:
    width = 23.6
    length = 152.4
    bed_to_top_up = 18.08
    bed_to_top_down = 7.54


class BlackKey:
    width = 10.2
    length = 101.6
    bed_to_top_up = WhiteKey.bed_to_top_up + 11.9
    bed_to_top_down = WhiteKey.bed_to_top_up + 3.6
    off_center = 3.5


KeyPlatform.bed_to_top = (
    BlackKey.bed_to_top_up
    + KeyPlatform.clearance
    + KeyPlatform.thickness
    - KeyPlatform.silicone_feet_thickness
)


class PlungerExtension:
    head_diameter = 20.3
    head_thickness = 2
    head_fuzz_thickness = 4
    white_key_stem_length = (
        KeyPlatform.bed_to_top
        + Solenoid.height
        - WhiteKey.bed_to_top_down
        - head_fuzz_thickness
        - head_thickness
        - Plunger.height / 2
    )
    black_key_stem_length = (
        KeyPlatform.bed_to_top
        + Solenoid.height
        - BlackKey.bed_to_top_down
        - head_fuzz_thickness
        - head_thickness
        - Plunger.height / 2
    )


class ControllerPCB:
    length = 127.6
    width = 40.9275
    thickness = 1.57
    center_y = 132


class PCBSupport:
    offset = 3
    height = 15


PCBSupport.inner_y = (
    ControllerPCB.center_y - ControllerPCB.width / 2 + PCBSupport.offset
)
PCBSupport.outer_y = (
    ControllerPCB.center_y + ControllerPCB.width / 2 - PCBSupport.offset
)


def self_tapping_hole(loc: cq.Location, radius: float, depth: float) -> cq.Solid:
    ridge_depth = radius * 0.19
    ridge_radius = radius * 0.625
    sth = cq.Workplane()
    sth = sth.sketch()
    sth = sth.circle(radius)
    sth = sth.circle(radius + ridge_radius - ridge_depth, mode="c", tag="outer")
    sth = sth.wires(tag="outer")
    sth = sth.distribute(3)
    sth = sth.circle(ridge_radius, mode="s")
    sth = sth.finalize()
    sth = sth.extrude(-depth)
    return sth.val().located(loc)


def export_stl(model, name):
    pwd = os.path.dirname(os.path.abspath(__file__))
    exporters.export(model, f"{pwd}/{name}.stl")
