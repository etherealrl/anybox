namespace Anybox.Medius;

/// <summary>a medius call that failed, with the library's text. <see cref="MediusStatus"/> is medius.h's number for
/// it and <see cref="ProtoVer"/> the protocol byte a box on another protocol answered with. it's an
/// <see cref="AnyboxException"/>, so one catch covers the whole package</summary>
public class MediusException(Medius.Status status, string message, byte protoVer)
    : AnyboxException(message.Length > 0 ? message : status.ToString())
{
    public Medius.Status MediusStatus => status;
    public byte ProtoVer => protoVer;

    internal static MediusException For(Medius.Status status, string message, byte protoVer, MediusUpdateError? update = null) => status switch
    {
        Medius.Status.ErrBadProtoVer => new BadProtoVerException(message, protoVer),
        Medius.Status.ErrCaptureNotApplicable => new CaptureNotApplicableException(message, protoVer),
        Medius.Status.ErrCatchTableFull => new CatchTableFullException(message, protoVer),
        Medius.Status.ErrClipFrameCount => new ClipFrameCountException(message, protoVer),
        Medius.Status.ErrClipFrameTooLong => new ClipFrameTooLongException(message, protoVer),
        Medius.Status.ErrClipPacketTrigger => new ClipPacketTriggerException(message, protoVer),
        Medius.Status.ErrClipTransferData => new ClipTransferDataException(message, protoVer),
        Medius.Status.ErrDisconnected => new DisconnectedException(message, protoVer),
        Medius.Status.ErrEmptySubscription => new EmptySubscriptionException(message, protoVer),
        Medius.Status.ErrFrameTooLong => new FrameTooLongException(message, protoVer),
        Medius.Status.ErrHalfEdgeInputFilter => new HalfEdgeInputFilterException(message, protoVer),
        Medius.Status.ErrImperfectRequired => new ImperfectRequiredException(message, protoVer),
        Medius.Status.ErrInvalidArg => new InvalidArgException(message, protoVer),
        Medius.Status.ErrIo => new IoException(message, protoVer),
        Medius.Status.ErrLockScaleRange => new LockScaleRangeException(message, protoVer),
        Medius.Status.ErrLockScaleUsage => new LockScaleUsageException(message, protoVer),
        Medius.Status.ErrNotAnInputFilter => new NotAnInputFilterException(message, protoVer),
        Medius.Status.ErrNotFound => new NotFoundException(message, protoVer),
        Medius.Status.ErrNoReply => new NoReplyException(message, protoVer),
        Medius.Status.ErrPanic => new PanicException(message, protoVer),
        Medius.Status.ErrQueryTimeout => new QueryTimeoutException(message, protoVer),
        Medius.Status.ErrRawDirection => new RawDirectionException(message, protoVer),
        Medius.Status.ErrRelativeDirection => new RelativeDirectionException(message, protoVer),
        Medius.Status.ErrReservedId => new ReservedIdException(message, protoVer),
        Medius.Status.ErrRewriteActionClass => new RewriteActionClassException(message, protoVer),
        Medius.Status.ErrRewriteMaskLength => new RewriteMaskLengthException(message, protoVer),
        Medius.Status.ErrRewriteMatchTooLong => new RewriteMatchTooLongException(message, protoVer),
        Medius.Status.ErrRewritePayloadTooLarge => new RewritePayloadTooLargeException(message, protoVer),
        Medius.Status.ErrRewritePoolFull => new RewritePoolFullException(message, protoVer),
        Medius.Status.ErrRewriteTableFull => new RewriteTableFullException(message, protoVer),
        Medius.Status.ErrTransformOpFields => new TransformOpFieldsException(message, protoVer),
        Medius.Status.ErrTransformTableFull => new TransformTableFullException(message, protoVer),
        Medius.Status.ErrUpdate => new UpdateException(message, protoVer, update?.op, update?.status, update?.arg),
        Medius.Status.ErrWildcardNotInput => new WildcardNotInputException(message, protoVer),
        _ => new MediusException(status, message, protoVer),
    };
}

public sealed class BadProtoVerException(string message, byte protoVer) : MediusException(Medius.Status.ErrBadProtoVer, message, protoVer);

public sealed class CaptureNotApplicableException(string message, byte protoVer) : MediusException(Medius.Status.ErrCaptureNotApplicable, message, protoVer);

public sealed class CatchTableFullException(string message, byte protoVer) : MediusException(Medius.Status.ErrCatchTableFull, message, protoVer);

public sealed class ClipFrameCountException(string message, byte protoVer) : MediusException(Medius.Status.ErrClipFrameCount, message, protoVer);

public sealed class ClipFrameTooLongException(string message, byte protoVer) : MediusException(Medius.Status.ErrClipFrameTooLong, message, protoVer);

public sealed class ClipPacketTriggerException(string message, byte protoVer) : MediusException(Medius.Status.ErrClipPacketTrigger, message, protoVer);

public sealed class ClipTransferDataException(string message, byte protoVer) : MediusException(Medius.Status.ErrClipTransferData, message, protoVer);

public sealed class DisconnectedException(string message, byte protoVer) : MediusException(Medius.Status.ErrDisconnected, message, protoVer);

public sealed class EmptySubscriptionException(string message, byte protoVer) : MediusException(Medius.Status.ErrEmptySubscription, message, protoVer);

public sealed class FrameTooLongException(string message, byte protoVer) : MediusException(Medius.Status.ErrFrameTooLong, message, protoVer);

public sealed class HalfEdgeInputFilterException(string message, byte protoVer) : MediusException(Medius.Status.ErrHalfEdgeInputFilter, message, protoVer);

public sealed class ImperfectRequiredException(string message, byte protoVer) : MediusException(Medius.Status.ErrImperfectRequired, message, protoVer);

public sealed class InvalidArgException(string message, byte protoVer) : MediusException(Medius.Status.ErrInvalidArg, message, protoVer);

public sealed class IoException(string message, byte protoVer) : MediusException(Medius.Status.ErrIo, message, protoVer);

public sealed class LockScaleRangeException(string message, byte protoVer) : MediusException(Medius.Status.ErrLockScaleRange, message, protoVer);

public sealed class LockScaleUsageException(string message, byte protoVer) : MediusException(Medius.Status.ErrLockScaleUsage, message, protoVer);

public sealed class NotAnInputFilterException(string message, byte protoVer) : MediusException(Medius.Status.ErrNotAnInputFilter, message, protoVer);

public sealed class NotFoundException(string message, byte protoVer) : MediusException(Medius.Status.ErrNotFound, message, protoVer);

public sealed class NoReplyException(string message, byte protoVer) : MediusException(Medius.Status.ErrNoReply, message, protoVer);

public sealed class PanicException(string message, byte protoVer) : MediusException(Medius.Status.ErrPanic, message, protoVer);

public sealed class QueryTimeoutException(string message, byte protoVer) : MediusException(Medius.Status.ErrQueryTimeout, message, protoVer);

public sealed class RawDirectionException(string message, byte protoVer) : MediusException(Medius.Status.ErrRawDirection, message, protoVer);

public sealed class RelativeDirectionException(string message, byte protoVer) : MediusException(Medius.Status.ErrRelativeDirection, message, protoVer);

public sealed class ReservedIdException(string message, byte protoVer) : MediusException(Medius.Status.ErrReservedId, message, protoVer);

public sealed class RewriteActionClassException(string message, byte protoVer) : MediusException(Medius.Status.ErrRewriteActionClass, message, protoVer);

public sealed class RewriteMaskLengthException(string message, byte protoVer) : MediusException(Medius.Status.ErrRewriteMaskLength, message, protoVer);

public sealed class RewriteMatchTooLongException(string message, byte protoVer) : MediusException(Medius.Status.ErrRewriteMatchTooLong, message, protoVer);

public sealed class RewritePayloadTooLargeException(string message, byte protoVer) : MediusException(Medius.Status.ErrRewritePayloadTooLarge, message, protoVer);

public sealed class RewritePoolFullException(string message, byte protoVer) : MediusException(Medius.Status.ErrRewritePoolFull, message, protoVer);

public sealed class RewriteTableFullException(string message, byte protoVer) : MediusException(Medius.Status.ErrRewriteTableFull, message, protoVer);

public sealed class TransformOpFieldsException(string message, byte protoVer) : MediusException(Medius.Status.ErrTransformOpFields, message, protoVer);

public sealed class TransformTableFullException(string message, byte protoVer) : MediusException(Medius.Status.ErrTransformTableFull, message, protoVer);

/// <summary>an update op the box refused, or a confirm that ran out of time. <see cref="Op"/> is the op (0 begin,
/// 1 data, 2 end, 3 abort, 4 activate), <see cref="UpdateStatus"/> the box's status byte for it (0x1B is a chip
/// still on probation) and <see cref="Arg"/> what goes with that status. a confirm that ran out of time gives op 0
/// and 0x1B, the refusal the next update would get. all three are null when the library had no detail</summary>
public sealed class UpdateException(string message, byte protoVer, byte? op = null, byte? updateStatus = null, uint? arg = null)
    : MediusException(Medius.Status.ErrUpdate, message, protoVer)
{
    public byte? Op => op;
    public byte? UpdateStatus => updateStatus;
    public uint? Arg => arg;
}

public sealed class WildcardNotInputException(string message, byte protoVer) : MediusException(Medius.Status.ErrWildcardNotInput, message, protoVer);
