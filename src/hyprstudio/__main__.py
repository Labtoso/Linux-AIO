import sys


def main() -> int:
    if "--laptop-keys" in sys.argv[1:]:  # nach der Installation: Laptop-Tasten ohne Fenster einrichten
        from .laptopkeys import cli
        return cli()
    if "--launcher" in sys.argv[1:]:  # L Launcher, ohne den Rest der App zu laden
        from .applauncher import main as launcher_main
        return launcher_main([a for a in sys.argv if a != "--launcher"])
    from .gui import main as gui_main
    return gui_main(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
