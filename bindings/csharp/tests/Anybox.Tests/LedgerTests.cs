using System.Reflection;

namespace Anybox.Tests;

// bindings/conformance/anybox-crate.txt and medius-crate.txt map every public thing of the anybox and medius crates
// to a C# name, and the capi tests hold each ledger to its crate. this holds each C# name to the package, by
// reflection, so a name that's wrong or gone fails here
public sealed class LedgerTests
{
    static readonly Type[] Types = typeof(Device).Assembly.GetExportedTypes();

    static Type? Find(string ns, string name) => Types.FirstOrDefault(type => type.Namespace == ns && type.Name == name);

    // a C# 14 extension block compiles to a nested type per receiver, holding the members and a marker type whose
    // one method takes the receiver, so that's how an extension member is told apart from one on another type
    static bool Extends(Type receiver, string member) => Types
        .Where(type => type.IsAbstract && type.IsSealed)
        .SelectMany(type => type.GetNestedTypes())
        .Where(group => group.GetNestedTypes(BindingFlags.Public | BindingFlags.NonPublic)
            .SelectMany(marker => marker.GetMethods(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static))
            .Any(method => method.GetParameters() is [var only] && only.ParameterType == receiver))
        .Any(group => group.GetMember(member, BindingFlags.Public | BindingFlags.Static | BindingFlags.Instance).Length > 0);

    static bool Has(string ns, string name)
    {
        var (typeName, member) = name.IndexOf('.') is var dot and >= 0 ? (name[..dot], name[(dot + 1)..]) : (name, null);
        var type = Find(ns, typeName);
        if (type is null)
        {
            return false;
        }
        return member switch
        {
            null => true,
            "ctor" => type.GetConstructors().Length > 0,
            // every type has the ToString it got from object, so only one it declares counts
            "ToString" => type.GetMethod("ToString", Type.EmptyTypes)?.DeclaringType == type,
            _ => type.GetMember(member, BindingFlags.Public | BindingFlags.Static | BindingFlags.Instance).Length > 0 ||
                Extends(type, member),
        };
    }

    // the names a ledger's C# column has that the package doesn't
    static List<string> Missing(string file, string ns, int column, int fields) => MockLibrary.ReadFile($"bindings/conformance/{file}")
        .Split('\n')
        .Select((line, index) => (Line: index + 1, Fields: line.Split('|').Select(field => field.Trim()).ToArray()))
        .Where(row => row.Fields.Length >= fields && !row.Fields[0].StartsWith('#') && row.Fields[column] != "" &&
            !Has(ns, row.Fields[column]))
        .Select(row => $"line {row.Line} {row.Fields[column]}")
        .ToList();

    [Fact]
    public void EveryCSharpNameInTheMediusLedgerIsInThePackage()
    {
        var missing = Missing("medius-crate.txt", "Anybox.Medius", 2, 4);
        Assert.True(missing.Count == 0, $"medius-crate.txt names these for C# and the package doesn't have them: {string.Join(", ", missing)}");
    }

    [Fact]
    public void EveryCSharpNameInTheCoreLedgerIsInThePackage()
    {
        var missing = Missing("anybox-crate.txt", "Anybox", 3, 5);
        Assert.True(missing.Count == 0, $"anybox-crate.txt names these for C# and the package doesn't have them: {string.Join(", ", missing)}");
    }

    [Fact]
    public void TheLookupFindsAnExtensionAndRefusesWhatIsntThere()
    {
        Assert.True(Has("Anybox.Medius", "Direction.IsRelative"));
        Assert.True(Has("Anybox.Medius", "Device.MoveRel"));
        Assert.True(Has("Anybox.Medius", "Bearing.ctor"));
        Assert.True(Has("Anybox", "Clock.WrapsAfter"));
        Assert.True(Has("Anybox", "StableId.ToString"));
        Assert.False(Has("Anybox", "Device.ToString"));
        Assert.False(Has("Anybox.Medius", "Direction.MoveRel"));
        Assert.False(Has("Anybox.Medius", "Device.IsRelative"));
        Assert.False(Has("Anybox", "NoSuchType"));
    }
}
