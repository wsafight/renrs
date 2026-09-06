define config.name = "Engine Comparison"
define config.window_title = "Engine Comparison"
define config.screen_width = 1280
define config.screen_height = 720
define config.physical_width = 1280
define config.physical_height = 720
define config.framerate = 60
define config.developer = False
define config.autosave_on_quit = False
define config.default_text_cps = 0
define config.rollback_length = 256

init python:
    import json
    import os
    import time
    _preferences.gl_framerate = 60
    bench_output = os.environ["RENRS_COMPARISON_OUTPUT"]
    bench_mode = os.environ["RENRS_COMPARISON_SCENARIO"]
    bench_started = None
    bench_flips = []
    bench_original_draw = None

    def bench_draw(*args, **kwargs):
        result = bench_original_draw(*args, **kwargs)
        if bench_started is not None:
            bench_flips.append(time.monotonic())
        return result

    def bench_install():
        global bench_original_draw
        bench_original_draw = renpy.display.interface.draw_screen
        renpy.display.interface.draw_screen = bench_draw

    def bench_begin():
        global bench_started
        with open(bench_output.replace(".json", ".ready.json"), "w") as stream:
            json.dump({"pid": os.getpid(), "warmup_seconds": 2, "measurement_seconds": 8}, stream)
        bench_started = time.monotonic()
        renpy.show_screen("benchmark_finish")

    def bench_end():
        elapsed = time.monotonic() - bench_started
        intervals = sorted((b - a) * 1000 for a, b in zip(bench_flips, bench_flips[1:]))
        def percentile(p):
            return intervals[min(len(intervals) - 1, int(len(intervals) * p))] if intervals else None
        with open(bench_output, "w") as stream:
            json.dump({"engine": renpy.version_string, "elapsed_seconds": elapsed,
                "scene_redraws": len(bench_flips), "draw_p50_ms": percentile(.5), "draw_p95_ms": percentile(.95)}, stream)
        renpy.screenshot(bench_output.replace(".json", ".start.png"))
        renpy.show_screen("benchmark_capture_finish")

    def bench_capture_end():
        renpy.screenshot(bench_output.replace(".json", ".end.png"))
        renpy.quit(save=False)

screen benchmark_timer():
    timer 2.0 action Function(bench_begin)

screen benchmark_finish():
    timer 8.0 action Function(bench_end)

screen benchmark_capture_finish():
    timer 0.5 action Function(bench_capture_end)

screen say(who, what):
    window:
        id "window"
        xpos 42 ypos 488 xsize 1196 ysize 196
        anchor (0.0, 0.0)
        background Solid("#111318f7")
        padding (34, 45)
        text what id "what" font "font.ttf" size 30 color "#f8fafc"

transform bench_still:
    size (440, 650)
    pos (84, 720)
    anchor (0.0, 1.0)

transform bench_motion:
    size (440, 650)
    pos (84, 720)
    anchor (0.0, 1.0)
    linear 30.0 xpos 484

label main_menu:
    jump start

label start:
    $ bench_install()
    scene expression Transform("studio.png", size=(1280, 720))
    show screen benchmark_timer
    if bench_mode == "motion":
        show expression "mira.png" at bench_motion
        $ renpy.pause(30.0, hard=True)
    else:
        show expression "mira.png" at bench_still
        "The receiver found another signal. Same time, same impossible frequency."
    return
