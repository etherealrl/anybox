namespace Anybox.Medius;

/// <summary>what a <see cref="CatchFilter"/> addresses. 0 to 3 are the lock and inject classes, 4 to 11 are byte
/// oriented traffic</summary>
public enum CatchClass : byte
{
    Button = Constants.CatchClassBtn,
    Key = Constants.CatchClassKey,
    Media = Constants.CatchClassMedia,
    Axis = Constants.CatchClassAxis,
    HidIn = Constants.CatchClassHidIn,
    HidOut = Constants.CatchClassHidOut,
    VendorInterrupt = Constants.CatchClassVendorInterrupt,
    VendorBulk = Constants.CatchClassVendorBulk,
    Control = Constants.CatchClassControl,
    Emit = Constants.CatchClassEmit,
    Bus = Constants.CatchClassBus,
    ClipTransfer = Constants.CatchClassClipTransfer,
}

/// <summary>the byte oriented half of the catch classes, for <see cref="CatchFilter.Traffic"/></summary>
public enum TrafficClass : byte
{
    HidIn = CatchClass.HidIn,
    HidOut = CatchClass.HidOut,
    VendorInterrupt = CatchClass.VendorInterrupt,
    VendorBulk = CatchClass.VendorBulk,
    Control = CatchClass.Control,
    Emit = CatchClass.Emit,
    Bus = CatchClass.Bus,
    ClipTransfer = CatchClass.ClipTransfer,
}

/// <summary>the bootloader's state for the image a chip runs. one this package has no name for comes through as its
/// number, like Rust's <c>Unknown(n)</c></summary>
public enum ImageState : byte
{
    New = 0,
    PendingVerify = 1,
    Valid = 2,
    Invalid = 3,
    Aborted = 4,
    Unknown = 0xFF,
}

/// <summary>which chip a firmware update goes to</summary>
public enum UpdateTarget : byte
{
    Device = 0,
    Host = 1,
}

/// <summary>how a catch class reads a <see cref="Direction"/>: the edge of a press or a release, the sign of a delta,
/// or which way traffic flows</summary>
public enum DirectionMeaning
{
    Edge,
    Sign,
    Flow,
}

public static class EnumExtensions
{
    extension(Direction direction)
    {
        /// <summary>whether it's measured against the bearing instead of a fixed sign</summary>
        public bool IsRelative => direction is Direction.With or Direction.Against;

        /// <summary>whether the two meet, which is when either one is both or they're the same</summary>
        public bool Admits(Direction other) => direction == Direction.Both || other == Direction.Both || direction == other;

        /// <summary>the sign of a delta, both for 0</summary>
        public static Direction OfDelta(short delta) => delta switch
        {
            > 0 => Direction.Positive,
            < 0 => Direction.Negative,
            _ => Direction.Both,
        };

        /// <summary>the direction a clip trigger's edge stands for</summary>
        public static Direction From(Edge edge) => edge switch
        {
            Edge.Both => Direction.Both,
            Edge.Press => Direction.Press,
            Edge.Release => Direction.Release,
            _ => throw new ArgumentOutOfRangeException(nameof(edge), $"{edge} isn't an Edge this package knows"),
        };
    }

    extension(CatchClass catchClass)
    {
        /// <summary>how this class reads a direction</summary>
        public DirectionMeaning DirectionMeaning => catchClass switch
        {
            CatchClass.Button or CatchClass.Key or CatchClass.Media => DirectionMeaning.Edge,
            CatchClass.Axis => DirectionMeaning.Sign,
            _ => DirectionMeaning.Flow,
        };

        public static CatchClass From(Class usageClass) => usageClass switch
        {
            Class.Button => CatchClass.Button,
            Class.Key => CatchClass.Key,
            Class.Media => CatchClass.Media,
            _ => throw new ArgumentOutOfRangeException(nameof(usageClass), $"{usageClass} isn't a Class this package knows"),
        };

        // each traffic class is defined as its catch class, so the cast is exact
        public static CatchClass From(TrafficClass trafficClass) => (CatchClass)(byte)trafficClass;

        /// <summary>a parsed input class, it arrives decoded with no packet, so a capture doesn't apply</summary>
        public bool IsInput
        {
            get
            {
                MediusLibrary.Ensure();
                return MediusImports.medius_catch_class_is_input((byte)catchClass) != 0;
            }
        }

        /// <summary>one of the eight byte oriented traffic classes</summary>
        public bool IsTraffic
        {
            get
            {
                MediusLibrary.Ensure();
                return MediusImports.medius_catch_class_is_traffic((byte)catchClass) != 0;
            }
        }
    }

    extension(TrafficClass)
    {
        /// <summary>the traffic class of a catch class, null for the four input classes</summary>
        public static TrafficClass? From(CatchClass catchClass) => catchClass.IsTraffic ? (TrafficClass)(byte)catchClass : null;
    }

    extension(ImageState state)
    {
        /// <summary>while the chip hasn't confirmed the image it booted. it refuses another update until then, since
        /// that image could still be rolled back</summary>
        public bool IsPending => state == ImageState.PendingVerify;

        /// <summary>like Rust's Display, "pending-verify", and "unknown(66)" for a state with no name</summary>
        public string Text => state switch
        {
            ImageState.New => "new",
            ImageState.PendingVerify => "pending-verify",
            ImageState.Valid => "valid",
            ImageState.Invalid => "invalid",
            ImageState.Aborted => "aborted",
            _ => $"unknown({(byte)state})",
        };
    }

    extension(DeviceKind kind)
    {
        /// <summary>like Rust's Display, "keyboard", "mouse" or "unknown"</summary>
        public string Text => kind switch
        {
            DeviceKind.Keyboard => "keyboard",
            DeviceKind.Mouse => "mouse",
            _ => "unknown",
        };
    }

    extension(UpdateTarget target)
    {
        /// <summary>like Rust's Display, "device" or "host"</summary>
        public string Text => target switch
        {
            UpdateTarget.Device => "device",
            UpdateTarget.Host => "host",
            _ => $"{(byte)target}",
        };
    }

    extension(Key key)
    {
        /// <summary>one of the eight modifiers, the left and right ctrl, shift, alt and gui</summary>
        public bool IsModifier => (byte)key is >= 0xE0 and <= 0xE7;
    }

    extension(RewriteClass rewriteClass)
    {
        /// <summary>the control class, the only one that can answer or rewrite a native reply</summary>
        public bool IsControl => rewriteClass == RewriteClass.Control;
    }

    extension(RewriteAction action)
    {
        /// <summary>whether a rule with this action needs a payload</summary>
        public bool CarriesPayload => action is RewriteAction.Patch or RewriteAction.Replace or RewriteAction.Answer or
            RewriteAction.ReplyPatch or RewriteAction.ReplyReplace;

        /// <summary>whether the box takes this action on that class, the same check it makes itself. drop is for the
        /// report classes only, answer, stall, nak and the reply rewrites for control only</summary>
        public bool IsValidFor(RewriteClass rewriteClass) => action switch
        {
            RewriteAction.Pass or RewriteAction.Patch or RewriteAction.Replace => true,
            RewriteAction.Drop => !rewriteClass.IsControl && rewriteClass != RewriteClass.Any,
            RewriteAction.Answer or RewriteAction.Stall or RewriteAction.Nak or RewriteAction.ReplyPatch or
                RewriteAction.ReplyReplace => rewriteClass.IsControl,
            _ => false,
        };
    }

    extension(TransformOp op)
    {
        /// <summary>whether the op takes that source and destination, the same check the box makes. one field as both
        /// ends is refused, and only the box knows whether the clone declares them</summary>
        public bool Admits(LockTarget source, LockTarget dest)
        {
            if (source == dest)
            {
                return false;
            }
            var axes = source.Kind != LockTargetKind.Usage && dest.Kind != LockTargetKind.Usage;
            return op switch
            {
                TransformOp.Swap => axes,
                TransformOp.Remap => axes || source.Input is { Kind: Class.Button } && dest.Input is not null,
                _ => false,
            };
        }
    }

    extension(TransferStatus status)
    {
        public bool IsOk => status == TransferStatus.Ok;
    }
}
