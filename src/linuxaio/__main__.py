import sys


def main() -> int:
    # Der Root-Helfer darf kein Qt laden (läuft per pkexec ohne Desktop-Umgebung)
    if len(sys.argv) > 1 and sys.argv[1] == "--helper":
        from .helper import main as helper_main
        return helper_main(sys.argv[2:])
    from . import usercmd  # noqa: F401  (setzt den Testmodus auch für Hypr Studio, bevor es geladen wird)
    from .gui import main as gui_main
    return gui_main(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
