// the package as an app gets it, with the library from inside it and no ANYBOX_LIB
using Anybox;

if (Environment.GetEnvironmentVariable("ANYBOX_LIB") is { Length: > 0 })
{
    throw new InvalidOperationException("unset ANYBOX_LIB, the smoke test is about the library inside the package");
}
Console.WriteLine($"anybox {Library.Version}, C ABI {Library.AbiVersion}");
if (Library.HasMock)
{
    throw new InvalidOperationException("the packaged library has the mock in it");
}
Console.WriteLine($"candidates: {string.Join(", ", Device.Candidates())}");
