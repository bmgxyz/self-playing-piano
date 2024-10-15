import cadquery as cq
from common import *

steel_support = (
    cq.Workplane("XZ")
    # Back wall
    .rect(
        SteelSupport.outer_arch_width,
        SteelSupport.height,
        centered=(True,False)
    )
    .moveTo(SteelSupport.outer_arch_width / 2, 0)
    .rect(
        SteelSupport.foot_width,
        SteelSupport.wall_thickness,
        centered=False
    )
    .moveTo(-SteelSupport.outer_arch_width / 2, 0)
    .rect(
        -SteelSupport.foot_width,
        SteelSupport.wall_thickness,
        centered=False
    )
    .extrude(SteelSupport.wall_thickness)
    # Inner arch
    .faces("<Y")
    .workplane()
    .move(-Solenoid.outer_diameter / 2, 0)
    .line(0, Solenoid.height)
    .line(Solenoid.outer_diameter, 0)
    .line(0, -Solenoid.height)
    .line(SteelSupport.wall_thickness, 0)
    .line(0, Solenoid.height + SteelSupport.wall_thickness)
    .line(
        -SteelSupport.wall_thickness
        - Solenoid.outer_diameter
        - SteelSupport.wall_thickness, 0
    )
    .line(0, -Solenoid.height - SteelSupport.wall_thickness)
    .close()
    # Outer arch and feet
    .workplane()
    .move(
        -SteelSupport.outer_arch_width / 2
        + SteelSupport.wall_thickness, 0
    )
    .line(0, SteelSupport.height - SteelSupport.wall_thickness)
    .line(
        SteelSupport.outer_arch_width
        - 2 * SteelSupport.wall_thickness, 0
    )
    .line(0, -SteelSupport.height + SteelSupport.wall_thickness)
    .line(SteelSupport.wall_thickness + SteelSupport.foot_width, 0)
    .line(0, SteelSupport.wall_thickness)
    .line(-SteelSupport.foot_width, 0)
    .line(0, SteelSupport.height - SteelSupport.wall_thickness)
    .line(-SteelSupport.outer_arch_width, 0)
    .line(0, -SteelSupport.height + SteelSupport.wall_thickness)
    .line(-SteelSupport.foot_width, 0)
    .line(0, -SteelSupport.wall_thickness)
    .close()
    .extrude(SteelSupport.depth_to_wall)
    # Plunger hole
    .faces(">Z")
    .workplane()
    .moveTo(0, -(SteelSupport.depth_to_wall / 2))
    .circle(Plunger.hole_diameter / 2)
    .cutThruAll()
    # Screw holes
    .faces("<Z")
    .workplane()
    .pushPoints([
        (
            (SteelSupport.outer_arch_width + SteelSupport.foot_width) / 2,
            SteelSupport.depth_to_wall / 2
        ),
        (
            -(SteelSupport.outer_arch_width + SteelSupport.foot_width) / 2,
            SteelSupport.depth_to_wall / 2
        )
    ])
    .circle(M3Screw.hole_diameter / 2)
    .cutThruAll()
)

export_stl(steel_support, "steel-support")
