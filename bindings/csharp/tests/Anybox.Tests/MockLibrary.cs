using System.Diagnostics;

[assembly: AssemblyFixture(typeof(Anybox.Tests.MockLibrary))]

namespace Anybox.Tests;

// builds the capi with the mock and points the package at it before any test touches the library
public sealed class MockLibrary
{
    public static readonly string Workspace = FindWorkspace();

    public MockLibrary()
    {
        if (Environment.GetEnvironmentVariable("ANYBOX_LIB") is { Length: > 0 })
        {
            return;
        }
        using var cargo = Process.Start(new ProcessStartInfo("cargo", "build -p anybox-capi --features mock --locked")
        {
            WorkingDirectory = Workspace,
        })!;
        cargo.WaitForExit();
        Assert.Equal(0, cargo.ExitCode);
        // cargo takes a relative CARGO_TARGET_DIR from where it runs, the workspace
        var target = Path.Combine(Workspace, Environment.GetEnvironmentVariable("CARGO_TARGET_DIR") ?? "target");
        var file = OperatingSystem.IsWindows() ? "anybox.dll" : "libanybox.so";
        Environment.SetEnvironmentVariable("ANYBOX_LIB", Path.Combine(target, "debug", file));
    }

    static string FindWorkspace()
    {
        for (var directory = new DirectoryInfo(AppContext.BaseDirectory); directory is not null; directory = directory.Parent)
        {
            if (File.Exists(Path.Combine(directory.FullName, "Cargo.toml")) &&
                Directory.Exists(Path.Combine(directory.FullName, "bindings", "c")))
            {
                return directory.FullName;
            }
        }
        throw new InvalidOperationException($"no anybox workspace above {AppContext.BaseDirectory}");
    }

    public static string ReadFile(string path) => File.ReadAllText(Path.Combine(Workspace, path));
}

// these swap what the whole process shares (the native calls, ANYBOX_LIB), so they run with nothing else
[CollectionDefinition(Name, DisableParallelization = true)]
public sealed class Alone
{
    public const string Name = "alone";
}
