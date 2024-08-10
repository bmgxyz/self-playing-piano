import cadquery as cq
from common import *

steel_support = (
    cq.Workplane("XZ")
    # Back wall
    .rect(
        SolenoidSupport.outer_arch_width,
        SolenoidSupport.height,
        centered=(True,False)
    )
    .moveTo(SolenoidSupport.outer_arch_width / 2, 0)
    .rect(
        SolenoidSupport.foot_width,
        SolenoidSupport.wall_thickness,
        centered=False
    )
    .moveTo(-SolenoidSupport.outer_arch_width / 2, 0)
    .rect(
        -SolenoidSupport.foot_width,
        SolenoidSupport.wall_thickness,
        centered=False
    )
    .extrude(SolenoidSupport.wall_thickness)
    # Inner arch
    .faces("<Y")
    .workplane()
    .move(-Solenoid.outer_diameter / 2, 0)
    .line(0, Solenoid.height)
    .line(Solenoid.outer_diameter, 0)
    .line(0, -Solenoid.height)
    .line(SolenoidSupport.wall_thickness, 0)
    .line(0, Solenoid.height + SolenoidSupport.wall_thickness)
    .line(
        -SolenoidSupport.wall_thickness
        - Solenoid.outer_diameter
        - SolenoidSupport.wall_thickness, 0
    )
    .line(0, -Solenoid.height - SolenoidSupport.wall_thickness)
    .close()
    # Outer arch and feet
    .workplane()
    .move(
        -SolenoidSupport.outer_arch_width / 2
        + SolenoidSupport.wall_thickness, 0
    )
    .line(0, SolenoidSupport.height - SolenoidSupport.wall_thickness)
    .line(
        SolenoidSupport.outer_arch_width
        - 2 * SolenoidSupport.wall_thickness, 0
    )
    .line(0, -SolenoidSupport.height + SolenoidSupport.wall_thickness)
    .line(SolenoidSupport.wall_thickness + SolenoidSupport.foot_width, 0)
    .line(0, SolenoidSupport.wall_thickness)
    .line(-SolenoidSupport.foot_width, 0)
    .line(0, SolenoidSupport.height - SolenoidSupport.wall_thickness)
    .line(-SolenoidSupport.outer_arch_width, 0)
    .line(0, -SolenoidSupport.height + SolenoidSupport.wall_thickness)
    .line(-SolenoidSupport.foot_width, 0)
    .line(0, -SolenoidSupport.wall_thickness)
    .close()
    .extrude(Solenoid.outer_diameter)
    # Plunger hole
    .faces(">Z")
    .workplane()
    .moveTo(0, -(Solenoid.outer_diameter / 2 + SolenoidSupport.wall_thickness))
    .circle(Plunger.hole_diameter / 2)
    .cutThruAll()
    # Screw holes
    .faces("<Z")
    .workplane()
    .pushPoints([
        (
            (SolenoidSupport.outer_arch_width + SolenoidSupport.foot_width) / 2,
             SolenoidSupport.wall_thickness + Solenoid.outer_diameter / 2
        ),
        (
            -(SolenoidSupport.outer_arch_width + SolenoidSupport.foot_width) / 2,
            SolenoidSupport.wall_thickness + Solenoid.outer_diameter / 2
        )
    ])
    .circle(M3Screw.hole_diameter / 2)
    .cutThruAll()
    # Wire hole
    .faces(">Y")
    .workplane(invert=True)
    .tag("back")
    .moveTo(0, -SolenoidSupport.wall_thickness * 1.5)
    .rect(SolenoidSupport.outer_arch_width, SolenoidSupport.wall_thickness)
    .cutBlind(SolenoidSupport.wall_thickness)
    .workplaneFromTagged("back")
    .moveTo(0, -SolenoidSupport.wall_thickness * 1.5)
    .rect(
        Solenoid.outer_diameter
        + SolenoidSupport.wall_thickness * 2,
        SolenoidSupport.wall_thickness
    )
    .cutBlind(SolenoidSupport.wall_thickness * 2)
)

export_stl(steel_support, "steel-support")
