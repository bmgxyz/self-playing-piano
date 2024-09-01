import cadquery as cq
from cadquery import exporters
import os

# All dimensions are in millimeters

class Plunger:
    diameter = 9.525 # 3/8"
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

class SteelSupport:
    wall_thickness = 1.2
    steel_tolerance = 0.2
    height = (
        Solenoid.height
        + wall_thickness * 2
        + Steel.thickness
        + steel_tolerance
    )
    outer_arch_width = (
        Solenoid.outer_diameter
        + wall_thickness * 4
        + Steel.thickness * 2
        + steel_tolerance * 2
    )
    screw_hole_separation = outer_arch_width + M3Screw.hole_buffer_diameter
    foot_width = screw_hole_separation - outer_arch_width
    wire_hole_height = 2

class KeyPlatform:
    clearance = 5
    thickness = 2
    base_depth = 24
    length = base_depth + 170

class PlatformRib:
    width = 2.4
    thickness = 12
    length = KeyPlatform.length - 1.5 * thickness

class WhiteKey:
    width = 23.6
    bed_to_top_up = 18.08
    bed_to_top_down = 7.54
    plunger_hole_offset = (
        KeyPlatform.base_depth
        + M3Screw.hole_buffer_diameter
        + SteelSupport.screw_hole_separation / 2
    )

class BlackKey:
    width = 10.2
    bed_to_top_up = WhiteKey.bed_to_top_up + 11.9
    bed_to_top_down = WhiteKey.bed_to_top_up + 3.6
    plunger_hole_offset = WhiteKey.plunger_hole_offset + 55
    off_center = 3.5

KeyPlatform.bed_to_top = (
    BlackKey.bed_to_top_up
    + KeyPlatform.clearance
    + KeyPlatform.thickness
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

def export_stl(model,name):
    pwd = os.path.dirname(os.path.abspath(__file__))
    exporters.export(model, f"{pwd}/{name}.stl")
