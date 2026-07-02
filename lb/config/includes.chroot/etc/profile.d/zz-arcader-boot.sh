if [ "${USER:-$(id -un)}" != "arcader" ]; then
  return 0 2>/dev/null
fi
if [ "$(tty 2>/dev/null)" != "/dev/tty1" ]; then
  return 0 2>/dev/null
fi
if [ -n "${DISPLAY:-}" ] || [ -n "${WAYLAND_DISPLAY:-}" ]; then
  return 0 2>/dev/null
fi

if [ "${XDG_VTNR:-1}" = "1" ]; then
  exec startx -- -quiet -nolisten tcp -background none vt1 >/dev/null 2>&1
fi
