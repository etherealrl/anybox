using System.Diagnostics;
using System.Runtime.InteropServices;

namespace Anybox.Tests;

// the library only gets looked up once per process, and on Windows a lookup by name finds the one this process
// already loaded, so each refusal runs in a program of its own
public sealed class LoadingTests
{
    static string Run(string? library, int exitCode, params string[] arguments)
    {
        var program = Path.Combine(MockLibrary.Workspace, "bindings", "csharp", "tests", "Refusal", "refusal.cs");
        var start = new ProcessStartInfo(Environment.GetEnvironmentVariable("DOTNET_HOST_PATH") ?? "dotnet", ["run", program, "--", .. arguments])
        {
            RedirectStandardOutput = true,
            RedirectStandardError = true,
        };
        start.Environment["ANYBOX_LIB"] = library;
        using var run = Process.Start(start)!;
        var output = run.StandardOutput.ReadToEnd();
        var errors = run.StandardError.ReadToEnd();
        run.WaitForExit();
        Assert.True(run.ExitCode == exitCode, $"exit {run.ExitCode}: {output}{errors}");
        return output;
    }

    [Fact]
    public void ALibraryWithAnotherAbiIsRefused()
    {
        Library.Ensure();
        Library.Load(Library.AbiVersion);
        var error = Assert.Throws<AnyboxException>(() => Library.Load(Library.AbiVersion + 1));
        Assert.Contains($"speaks C ABI {Library.AbiVersion} and this package needs {Library.AbiVersion + 1}", error.Message);
        Assert.Null(error.Status);
    }

    // one test, so the runs don't build the same program at the same time
    [Fact]
    public void ALibraryThatCantBeUsedSaysWhatToDo()
    {
        var missing = Path.Combine(Path.GetTempPath(), "nothing-here", "anybox.dll");
        Assert.Contains($"ANYBOX_LIB points at {missing}, which doesn't exist", Run(missing, 1));
        Assert.Contains("so reinstall it", Run(null, 1));
        var other = OperatingSystem.IsWindows()
            ? Path.Combine(Environment.SystemDirectory, "kernel32.dll")
            : Path.Combine(RuntimeEnvironment.GetRuntimeDirectory(), "libSystem.Native.so");
        Assert.Contains($"{other} isn't an anybox library", Run(other, 1));
        // a default Medius filter used first thing in a process still loads the library
        Assert.Equal("8", Run(Environment.GetEnvironmentVariable("ANYBOX_LIB"), 0, "default-filter").Trim());
    }
}
