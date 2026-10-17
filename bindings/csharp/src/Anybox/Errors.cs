namespace Anybox;

/// <summary>everything anybox throws. <see cref="Status"/> is the C ABI's number for it, and null for the
/// package's own errors, like a library it can't load. each one has the fields of its Rust error</summary>
public class AnyboxException(string message) : Exception(message)
{
    public virtual uint? Status => null;

    internal static AnyboxException For(
        uint status,
        string message,
        ErrorFields? fields = null,
        IReadOnlyList<string>? tried = null,
        IReadOnlyList<Incapable>? boxes = null)
    {
        var f = fields ?? new ErrorFields();
        return status switch
        {
            1 => new NotFoundException(message) { Tried = tried ?? [] },
            2 => new ClosedException(message),
            3 => new PortBusyException(message) { Port = f.Port ?? "" },
            4 => new PermissionDeniedException(message) { Port = f.Port ?? "" },
            5 => new NoBoxCanException(message) { Boxes = boxes ?? [] },
            6 => new WrongFamilyException(message)
            {
                Port = f.Port ?? "",
                Expected = f.ExpectedFamily ?? default,
                Found = f.FoundFamily,
            },
            7 => new WrongBoxException(message) { Port = f.Port ?? "", Expected = f.ExpectedId, Found = f.FoundId },
            8 => new WrongFerrumApiException(message)
            {
                Port = f.Port ?? "",
                Expected = f.ExpectedFerrumApi ?? default,
                Found = f.FoundFerrumApi ?? default,
            },
            9 => new FirmwareTooOldException(message)
            {
                Family = f.Family ?? default,
                Found = f.FoundFirmware,
                Needed = f.NeededFirmware,
            },
            10 => new LibraryTooOldException(message) { Family = f.Family ?? default, Found = f.FoundFirmware },
            11 => new UnsupportedException(message) { Action = f.Action, Family = f.Family, Reason = f.Text ?? "" },
            12 => new InvalidArgumentException(message) { Action = f.Action, Detail = f.Text ?? "" },
            13 => new RejectedException(message) { Action = f.Action ?? default, Detail = f.Text ?? "" },
            14 => new BoxTimeoutException(message) { Action = f.Action ?? default },
            15 => new DisconnectedException(message),
            16 => new BoxInterruptedException(message) { Action = f.Action ?? default },
            17 => new IoException(message) { Kind = f.IoKind ?? IoKind.Other, OsError = f.OsError },
            18 => new PanicException(message),
            21 => new RecvClosedException(message),
            _ => new UnknownException(message),
        };
    }
}

// the flat fields of a failure, copied out of the C detail before anything can replace it
internal sealed record ErrorFields
{
    public BoxAction? Action { get; init; }
    public Family? Family { get; init; }
    public Family? ExpectedFamily { get; init; }
    public Family? FoundFamily { get; init; }
    public string? Port { get; init; }
    public string? Text { get; init; }
    public StableId? ExpectedId { get; init; }
    public StableId? FoundId { get; init; }
    public Firmware? FoundFirmware { get; init; }
    public Firmware? NeededFirmware { get; init; }
    public FerrumApi? ExpectedFerrumApi { get; init; }
    public FerrumApi? FoundFerrumApi { get; init; }
    public IoKind? IoKind { get; init; }
    public int? OsError { get; init; }
}

/// <summary>a box that answered, with the actions it couldn't do</summary>
public sealed record Incapable(Snapshot Box, IReadOnlySet<BoxAction> Missing);

public sealed class NotFoundException(string message) : AnyboxException(message)
{
    public override uint? Status => 1;

    /// <summary>every port it looked at</summary>
    public IReadOnlyList<string> Tried { get; init; } = [];
}

public sealed class ClosedException(string message) : AnyboxException(message)
{
    public override uint? Status => 2;
}

public sealed class PortBusyException(string message) : AnyboxException(message)
{
    public override uint? Status => 3;
    public string Port { get; init; } = "";
}

public sealed class PermissionDeniedException(string message) : AnyboxException(message)
{
    public override uint? Status => 4;
    public string Port { get; init; } = "";
}

public sealed class NoBoxCanException(string message) : AnyboxException(message)
{
    public override uint? Status => 5;

    /// <summary>each box that answered, with what it couldn't do</summary>
    public IReadOnlyList<Incapable> Boxes { get; init; } = [];
}

public sealed class WrongFamilyException(string message) : AnyboxException(message)
{
    public override uint? Status => 6;
    public string Port { get; init; } = "";
    public Family Expected { get; init; }
    public Family? Found { get; init; }
}

public sealed class WrongBoxException(string message) : AnyboxException(message)
{
    public override uint? Status => 7;
    public string Port { get; init; } = "";
    public StableId? Expected { get; init; }
    public StableId? Found { get; init; }
}

public sealed class WrongFerrumApiException(string message) : AnyboxException(message)
{
    public override uint? Status => 8;
    public string Port { get; init; } = "";
    public FerrumApi Expected { get; init; }
    public FerrumApi Found { get; init; }
}

public sealed class FirmwareTooOldException(string message) : AnyboxException(message)
{
    public override uint? Status => 9;
    public Family Family { get; init; }
    public Firmware? Found { get; init; }
    public Firmware? Needed { get; init; }
}

public sealed class LibraryTooOldException(string message) : AnyboxException(message)
{
    public override uint? Status => 10;
    public Family Family { get; init; }
    public Firmware? Found { get; init; }
}

/// <summary><see cref="Action"/> and <see cref="Family"/> are null when the C ABI refused it on its own, like
/// <see cref="Device.Medius"/> on another box</summary>
public sealed class UnsupportedException(string message) : AnyboxException(message)
{
    public override uint? Status => 11;
    public BoxAction? Action { get; init; }
    public Family? Family { get; init; }
    public string Reason { get; init; } = "";
}

/// <summary><see cref="Action"/> is null when the package or the C ABI refused the value before anybox saw
/// it</summary>
public sealed class InvalidArgumentException(string message) : AnyboxException(message)
{
    public override uint? Status => 12;
    public BoxAction? Action { get; init; }
    public string Detail { get; init; } = "";
}

public sealed class RejectedException(string message) : AnyboxException(message)
{
    public override uint? Status => 13;
    public BoxAction Action { get; init; }
    public string Detail { get; init; } = "";
}

/// <summary>the box didn't answer in time, so a <see cref="Events.Recv"/> timing out is never this</summary>
public sealed class BoxTimeoutException(string message) : AnyboxException(message)
{
    public override uint? Status => 14;
    public BoxAction Action { get; init; }
}

public sealed class DisconnectedException(string message) : AnyboxException(message)
{
    public override uint? Status => 15;
}

/// <summary>the box reconnected or reset in the middle of the call, anybox already let go of it</summary>
public sealed class BoxInterruptedException(string message) : AnyboxException(message)
{
    public override uint? Status => 16;
    public BoxAction Action { get; init; }
}

/// <summary><see cref="OsError"/> is the operating system's own code, when it came from there</summary>
public sealed class IoException(string message) : AnyboxException(message)
{
    public override uint? Status => 17;
    public IoKind Kind { get; init; } = IoKind.Other;
    public int? OsError { get; init; }
}

/// <summary>anybox panicked inside the call. the device still works and can be closed, but what the call already
/// did on the box isn't known</summary>
public sealed class PanicException(string message) : AnyboxException(message)
{
    public override uint? Status => 18;
}

/// <summary>an error the C ABI has no number for yet, the message still says what happened</summary>
public sealed class UnknownException(string message) : AnyboxException(message)
{
    public override uint? Status => 19;
}

/// <summary>a receiver has nothing left, the device is gone and every event it queued was read. not
/// <see cref="ClosedException"/>, which is a call on a closed device or a disposed receiver, like in Rust</summary>
public sealed class RecvClosedException(string message) : AnyboxException(message)
{
    public override uint? Status => 21;
}
