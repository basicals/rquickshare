#!/usr/bin/env python3
"""Render a review-only GTK4 widget tree on an owned private display.

Run inside dbus-run-session and xvfb-run, never as the production application.
GtkBuilder source is shared with the prospective Rust/gtk-rs renderer.
"""
import argparse
from pathlib import Path
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page", choices=("nearby", "incoming", "transfers"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    import gi
    gi.require_version("Gtk", "4.0")
    gi.require_version("Gsk", "4.0")
    from gi.repository import Gio, Gtk
    root = Path(__file__).resolve().parents[1]
    app = Gtk.Application(application_id="io.github.morozhenko_apps.RQuickShare.Preview",
                          flags=Gio.ApplicationFlags.NON_UNIQUE)
    status = {"captured": False, "error": None}

    def activated(application):
        builder = Gtk.Builder.new_from_file(str(root / "docs/prototypes/native-panel.ui"))
        window = builder.get_object("panel")
        window.set_application(application)
        builder.get_object("pages").set_visible_child_name(args.page)
        provider = Gtk.CssProvider()
        provider.load_from_path(str(root / "docs/prototypes/native-panel.css"))
        Gtk.StyleContext.add_provider_for_display(window.get_display(), provider,
                                                  Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)

        def mapped(widget):
            def painted(clock):
                if status["captured"]:
                    return
                width, height = widget.get_width(), widget.get_height()
                if width <= 0 or height <= 0:
                    return
                try:
                    paintable = Gtk.WidgetPaintable.new(widget)
                    snapshot = Gtk.Snapshot.new()
                    paintable.snapshot(snapshot, float(width), float(height))
                    node = snapshot.to_node()
                    if node is None:
                        raise RuntimeError("Mapped panel did not produce a render node")
                    texture = widget.get_renderer().render_texture(node, None)
                    args.output.parent.mkdir(parents=True, exist_ok=True)
                    if not texture.save_to_png(str(args.output)):
                        raise RuntimeError("Native texture export failed")
                    print(f"GTK4 {args.page}: {width}x{height} -> {args.output}")
                    status["captured"] = True
                except Exception as error:
                    status["error"] = str(error)
                finally:
                    application.quit()
            widget.get_frame_clock().connect("after-paint", painted)
        window.connect("map", mapped)
        window.present()

    app.connect("activate", activated)
    app.run([sys.argv[0]])
    if status["error"]:
        raise SystemExit(status["error"])
    if not status["captured"]:
        raise SystemExit("No native panel was captured")


if __name__ == "__main__":
    main()
