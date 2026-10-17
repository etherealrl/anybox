# sourced on Windows by the scripts that build with MSVC. Git Bash has no cl, and it puts its own link.exe ahead of
# MSVC's on the path, so a bare cargo build can pick the wrong one. this finds Visual Studio with vswhere and points
# cargo at MSVC's linker, and msvc loads vcvars64 into the shell for the steps that call cl
vswhere="$(printenv 'ProgramFiles(x86)' || echo 'C:\Program Files (x86)')\Microsoft Visual Studio\Installer\vswhere.exe"
vs=''
if [ -f "$vswhere" ]; then
  vs="$("$vswhere" -latest -prerelease -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 \
    -property installationPath | tr -d '\r')" || true
fi
if [ -z "$vs" ]; then
  echo "msvc: no Visual Studio with the C++ tools found, install it (the Build Tools with \"Desktop development with C++\" are enough)" >&2
  exit 1
fi
vs="$(cygpath -u "$vs")"
tools="$(tr -d '\r\n ' < "$vs/VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt")"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER
CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="$(cygpath -w "$vs/VC/Tools/MSVC/$tools/bin/HostX64/x64/link.exe")"

# bash can't call a batch file, so cmd runs vcvars64 and this takes over the paths it set, which is all cl needs.
# only the steps that call cl load it, everything else builds the way it does without
msvc() {
  local name value
  while IFS='=' read -r name value; do
    value="${value%$'\r'}"
    case "${name^^}" in
      PATH) PATH="$(cygpath -up "$value")" ;;
      INCLUDE | LIB | LIBPATH) export "$name=$value" ;;
    esac
  done < <(printf 'call "%s" >nul 2>&1\r\nset\r\n' "$(cygpath -w "$vs/VC/Auxiliary/Build/vcvars64.bat")" | cmd.exe //d //q)
  command -v cl >/dev/null || { echo "msvc: vcvars64 ran but cl still isn't on the path" >&2; return 1; }
}
