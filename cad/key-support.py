from common import M3Screw, export_stl, run, Support, WhiteKey, Plunger, SolenoidSupport

from enum import StrEnum
import cadquery as cq


white_key_hole_position = (
    Support.inner_gap - SolenoidSupport.screw_hole_separation
) / 2
black_key_hole_position = (
    Support.inner_gap + SolenoidSupport.screw_hole_separation
) / 2


class KeyPattern(StrEnum):
    Octave = "Octave"
    Low = "Low"
    High = "High"

    @property
    def plunger_positions(self) -> list[tuple[float, float]]:
        match self:
            case KeyPattern.Octave:
                return [
                    (WhiteKey.width / 2 *  1, white_key_hole_position),
                    (WhiteKey.width / 2 *  2, black_key_hole_position),
                    (WhiteKey.width / 2 *  3, white_key_hole_position),
                    (WhiteKey.width / 2 *  4, black_key_hole_position),
                    (WhiteKey.width / 2 *  5, white_key_hole_position),
                    (WhiteKey.width / 2 *  6, black_key_hole_position),
                    (WhiteKey.width / 2 *  7, white_key_hole_position),
                    (WhiteKey.width / 2 *  9, white_key_hole_position),
                    (WhiteKey.width / 2 * 10, black_key_hole_position),
                    (WhiteKey.width / 2 * 11, white_key_hole_position),
                    (WhiteKey.width / 2 * 12, black_key_hole_position),
                    (WhiteKey.width / 2 * 13, white_key_hole_position),
                ]
            case KeyPattern.Low:
                return [
                    (WhiteKey.width / 2 * 1, white_key_hole_position),
                    (WhiteKey.width / 2 * 2, black_key_hole_position),
                    (WhiteKey.width / 2 * 3, white_key_hole_position),
                    (WhiteKey.width / 2 * 5, white_key_hole_position),
                    (WhiteKey.width / 2 * 6, black_key_hole_position),
                    (WhiteKey.width / 2 * 7, white_key_hole_position),
                    (WhiteKey.width / 2 * 8, black_key_hole_position),
                    (WhiteKey.width / 2 * 9, white_key_hole_position),
                ]
            case KeyPattern.High:
                return [
                    (WhiteKey.width / 2 * 1, white_key_hole_position),
                    (WhiteKey.width / 2 * 2, black_key_hole_position),
                    (WhiteKey.width / 2 * 3, white_key_hole_position),
                    (WhiteKey.width / 2 * 4, black_key_hole_position),
                    (WhiteKey.width / 2 * 5, white_key_hole_position),
                    (WhiteKey.width / 2 * 6, black_key_hole_position),
                    (WhiteKey.width / 2 * 7, white_key_hole_position),
                    (WhiteKey.width / 2 * 9, white_key_hole_position),
                ]
            case _:
                raise ValueError(f"Expected KeyPattern variant, got '{self}'")

    @property
    def width(self) -> float:
        match self:
            # subtract 1 mm here to give some tolerance between support modules
            case KeyPattern.Octave:
                return WhiteKey.width * 7 - 1
            case KeyPattern.Low:
                return WhiteKey.width * 5 - 1
            case KeyPattern.High:
                return WhiteKey.width * 5 - 1
            case _:
                raise ValueError(f"Expected KeyPattern variant, got '{self}'")


def plunger_to_screws(
    plunger_positions: list[tuple[float, float]],
) -> list[tuple[float, float]]:
    screw_positions = []
    for p in plunger_positions:
        screw_positions.append((p[0], p[1] + SolenoidSupport.screw_hole_separation / 2))
        screw_positions.append((p[0], p[1] - SolenoidSupport.screw_hole_separation / 2))
    return screw_positions


def make_key_support(pattern: KeyPattern) -> cq.Solid:
    key_support = Support.make_profile(pattern.width, chamfer=False)
    key_support = (
        key_support.moveTo(0, Support.height * 1 / 4)
        .rect(Support.depth, Support.height * 3 / 4, centered=False)
        .cutThruAll()
    )
    key_support = (
        key_support.faces("<Y")
        .workplane()
        .pushPoints([
            (                WhiteKey.width * 1 / 4, Support.height / 6),
            (pattern.width - WhiteKey.width * 1 / 4, Support.height / 6)
        ])
        .circle(M3Screw.hole_diameter / 2)
        .cutThruAll()
    )
    key_support = (
        key_support.faces("<Z")
        .workplane(invert=True)
        .transformed(
            offset=(
                0,
                Support.wall_thickness * 2 + Support.wood_gap_width,
                Support.wall_thickness,
            )
        )
        .pushPoints(pattern.plunger_positions)
        .hole(Plunger.hole_diameter)
        .pushPoints(plunger_to_screws(pattern.plunger_positions))
        .hole(M3Screw.hole_diameter)
    )
    return key_support

def build():
    return make_key_support(KeyPattern.Octave)


def export():
    export_stl(make_key_support(KeyPattern.Octave), "key-support-octave")
    export_stl(make_key_support(KeyPattern.Low), "key-support-low")
    export_stl(make_key_support(KeyPattern.High), "key-support-high")

run(build, export)
