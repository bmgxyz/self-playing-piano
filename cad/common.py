import cadquery as cq
from cadquery import exporters
import os

class Plunger:
    diameter = 9.525
    height = 42
    hole_diameter = diameter + 2

class Steel:
    thickness = 2

class Solenoid:
    height = 32
    outer_diameter = Plunger.hole_diameter + 12

class M3Screw:
    diameter = 3
    hole_diameter = 3.5
    hole_buffer_diameter = 10

class SolenoidSupport:
    wall_thickness = 0.6
    base_thickness = 1.8
    inner_diameter = Plunger.diameter + 0.8
    steel_tolerance = 0.2
    outer_arch_width = (
        Solenoid.outer_diameter
        + wall_thickness * 4
        + Steel.thickness * 2
        + steel_tolerance * 2
    )
    screw_hole_separation = outer_arch_width + M3Screw.hole_buffer_diameter
    foot_width = screw_hole_separation - outer_arch_width
    wall_thickness = 1.2
    height = (
        Solenoid.height
        + wall_thickness * 2
        + Steel.thickness
        + steel_tolerance
    )
    taper_fraction = 0.15
    taper_size = 5

class KeyPlatform:
    clearance = 5
    thickness = 2
    chamfer = 3
    base_depth = 24
    length = base_depth + chamfer + 152

class WhiteKey:
    width = 23.6
    bed_to_top_up = 18.08
    bed_to_top_down = 7.54
    plunger_hole_offset = (
        KeyPlatform.base_depth
        + KeyPlatform.chamfer
        + M3Screw.hole_buffer_diameter
        + SolenoidSupport.screw_hole_separation / 2
    )

class BlackKey:
    width = 10.2
    bed_to_top_up = WhiteKey.bed_to_top_up + 11.9
    bed_to_top_down = WhiteKey.bed_to_top_up + 3.6
    plunger_hole_offset = WhiteKey.plunger_hole_offset + 50

KeyPlatform.bed_to_top = (
    BlackKey.bed_to_top_up
    + KeyPlatform.clearance
    + KeyPlatform.chamfer
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