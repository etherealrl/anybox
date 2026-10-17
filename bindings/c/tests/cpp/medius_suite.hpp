// made by bindings/csharp/tools/gen-medius.cs from bindings/c/include/medius.h and its tables, don't edit it by hand.
// it goes in the Medius suite's runner after the overloads it builds on

inline Value write(Status value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Ok");
    case 1:
        return Value("ErrIo");
    case 2:
        return Value("ErrNotFound");
    case 3:
        return Value("ErrNoReply");
    case 4:
        return Value("ErrBadProtoVer");
    case 5:
        return Value("ErrQueryTimeout");
    case 6:
        return Value("ErrDisconnected");
    case 7:
        return Value("ErrFrameTooLong");
    case 8:
        return Value("ErrUpdate");
    case 9:
        return Value("ErrInvalidArg");
    case 10:
        return Value("ErrPanic");
    case 11:
        return Value("ErrUnknown");
    case 12:
        return Value("ErrCatchTableFull");
    case 13:
        return Value("ErrEmptySubscription");
    case 14:
        return Value("ErrCaptureNotApplicable");
    case 15:
        return Value("ErrNotAnInputFilter");
    case 16:
        return Value("ErrWildcardNotInput");
    case 17:
        return Value("ErrHalfEdgeInputFilter");
    case 18:
        return Value("ErrReservedId");
    case 19:
        return Value("ErrRelativeDirection");
    case 20:
        return Value("ErrLockScaleRange");
    case 21:
        return Value("ErrLockScaleUsage");
    case 22:
        return Value("ErrImperfectRequired");
    case 23:
        return Value("ErrRewriteMaskLength");
    case 24:
        return Value("ErrRewriteActionClass");
    case 25:
        return Value("ErrRewritePayloadTooLarge");
    case 26:
        return Value("ErrRewriteTableFull");
    case 27:
        return Value("ErrTransformOpFields");
    case 28:
        return Value("ErrTransformTableFull");
    case 29:
        return Value("ErrRawDirection");
    case 30:
        return Value("ErrClipFrameCount");
    case 31:
        return Value("ErrClipFrameTooLong");
    case 32:
        return Value("ErrClipTransferData");
    case 33:
        return Value("ErrClipPacketTrigger");
    case 34:
        return Value("ErrRewriteMatchTooLong");
    case 35:
        return Value("ErrRewritePoolFull");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, Status& out) {
    if (!value.is_string()) {
        out = static_cast<Status>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Ok") {
        out = Status::Ok;
        return;
    }
    if (name == "ErrIo") {
        out = Status::ErrIo;
        return;
    }
    if (name == "ErrNotFound") {
        out = Status::ErrNotFound;
        return;
    }
    if (name == "ErrNoReply") {
        out = Status::ErrNoReply;
        return;
    }
    if (name == "ErrBadProtoVer") {
        out = Status::ErrBadProtoVer;
        return;
    }
    if (name == "ErrQueryTimeout") {
        out = Status::ErrQueryTimeout;
        return;
    }
    if (name == "ErrDisconnected") {
        out = Status::ErrDisconnected;
        return;
    }
    if (name == "ErrFrameTooLong") {
        out = Status::ErrFrameTooLong;
        return;
    }
    if (name == "ErrUpdate") {
        out = Status::ErrUpdate;
        return;
    }
    if (name == "ErrInvalidArg") {
        out = Status::ErrInvalidArg;
        return;
    }
    if (name == "ErrPanic") {
        out = Status::ErrPanic;
        return;
    }
    if (name == "ErrUnknown") {
        out = Status::ErrUnknown;
        return;
    }
    if (name == "ErrCatchTableFull") {
        out = Status::ErrCatchTableFull;
        return;
    }
    if (name == "ErrEmptySubscription") {
        out = Status::ErrEmptySubscription;
        return;
    }
    if (name == "ErrCaptureNotApplicable") {
        out = Status::ErrCaptureNotApplicable;
        return;
    }
    if (name == "ErrNotAnInputFilter") {
        out = Status::ErrNotAnInputFilter;
        return;
    }
    if (name == "ErrWildcardNotInput") {
        out = Status::ErrWildcardNotInput;
        return;
    }
    if (name == "ErrHalfEdgeInputFilter") {
        out = Status::ErrHalfEdgeInputFilter;
        return;
    }
    if (name == "ErrReservedId") {
        out = Status::ErrReservedId;
        return;
    }
    if (name == "ErrRelativeDirection") {
        out = Status::ErrRelativeDirection;
        return;
    }
    if (name == "ErrLockScaleRange") {
        out = Status::ErrLockScaleRange;
        return;
    }
    if (name == "ErrLockScaleUsage") {
        out = Status::ErrLockScaleUsage;
        return;
    }
    if (name == "ErrImperfectRequired") {
        out = Status::ErrImperfectRequired;
        return;
    }
    if (name == "ErrRewriteMaskLength") {
        out = Status::ErrRewriteMaskLength;
        return;
    }
    if (name == "ErrRewriteActionClass") {
        out = Status::ErrRewriteActionClass;
        return;
    }
    if (name == "ErrRewritePayloadTooLarge") {
        out = Status::ErrRewritePayloadTooLarge;
        return;
    }
    if (name == "ErrRewriteTableFull") {
        out = Status::ErrRewriteTableFull;
        return;
    }
    if (name == "ErrTransformOpFields") {
        out = Status::ErrTransformOpFields;
        return;
    }
    if (name == "ErrTransformTableFull") {
        out = Status::ErrTransformTableFull;
        return;
    }
    if (name == "ErrRawDirection") {
        out = Status::ErrRawDirection;
        return;
    }
    if (name == "ErrClipFrameCount") {
        out = Status::ErrClipFrameCount;
        return;
    }
    if (name == "ErrClipFrameTooLong") {
        out = Status::ErrClipFrameTooLong;
        return;
    }
    if (name == "ErrClipTransferData") {
        out = Status::ErrClipTransferData;
        return;
    }
    if (name == "ErrClipPacketTrigger") {
        out = Status::ErrClipPacketTrigger;
        return;
    }
    if (name == "ErrRewriteMatchTooLong") {
        out = Status::ErrRewriteMatchTooLong;
        return;
    }
    if (name == "ErrRewritePoolFull") {
        out = Status::ErrRewritePoolFull;
        return;
    }
    throw std::runtime_error("Status has no member " + name);
}

inline Value write(EmitMode value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Learned");
    case 1:
        return Value("Interval");
    case 2:
        return Value("Fixed");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, EmitMode& out) {
    if (!value.is_string()) {
        out = static_cast<EmitMode>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Learned") {
        out = EmitMode::Learned;
        return;
    }
    if (name == "Interval") {
        out = EmitMode::Interval;
        return;
    }
    if (name == "Fixed") {
        out = EmitMode::Fixed;
        return;
    }
    throw std::runtime_error("EmitMode has no member " + name);
}

inline Value write(BearingMode value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("PerAxis");
    case 1:
        return Value("Vector");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, BearingMode& out) {
    if (!value.is_string()) {
        out = static_cast<BearingMode>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "PerAxis") {
        out = BearingMode::PerAxis;
        return;
    }
    if (name == "Vector") {
        out = BearingMode::Vector;
        return;
    }
    throw std::runtime_error("BearingMode has no member " + name);
}

inline Value write(RenderMode value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Off");
    case 1:
        return Value("Stock");
    case 2:
        return Value("Despiked");
    case 3:
        return Value("Unsmoothed");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, RenderMode& out) {
    if (!value.is_string()) {
        out = static_cast<RenderMode>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Off") {
        out = RenderMode::Off;
        return;
    }
    if (name == "Stock") {
        out = RenderMode::Stock;
        return;
    }
    if (name == "Despiked") {
        out = RenderMode::Despiked;
        return;
    }
    if (name == "Unsmoothed") {
        out = RenderMode::Unsmoothed;
        return;
    }
    throw std::runtime_error("RenderMode has no member " + name);
}

inline Value write(ControlStatus value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Ok");
    case 1:
        return Value("Stalled");
    case 2:
        return Value("Naked");
    case 3:
        return Value("Other");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, ControlStatus& out) {
    if (!value.is_string()) {
        out = static_cast<ControlStatus>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Ok") {
        out = ControlStatus::Ok;
        return;
    }
    if (name == "Stalled") {
        out = ControlStatus::Stalled;
        return;
    }
    if (name == "Naked") {
        out = ControlStatus::Naked;
        return;
    }
    if (name == "Other") {
        out = ControlStatus::Other;
        return;
    }
    throw std::runtime_error("ControlStatus has no member " + name);
}

inline Value write(BusEventKind value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Reset");
    case 1:
        return Value("Suspend");
    case 2:
        return Value("Resume");
    case 3:
        return Value("Configured");
    case 4:
        return Value("Deconfigured");
    case 5:
        return Value("SetInterface");
    case 6:
        return Value("DeviceAttached");
    case 7:
        return Value("DeviceDetached");
    case 8:
        return Value("CloneUp");
    case 9:
        return Value("CloneDown");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, BusEventKind& out) {
    if (!value.is_string()) {
        out = static_cast<BusEventKind>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Reset") {
        out = BusEventKind::Reset;
        return;
    }
    if (name == "Suspend") {
        out = BusEventKind::Suspend;
        return;
    }
    if (name == "Resume") {
        out = BusEventKind::Resume;
        return;
    }
    if (name == "Configured") {
        out = BusEventKind::Configured;
        return;
    }
    if (name == "Deconfigured") {
        out = BusEventKind::Deconfigured;
        return;
    }
    if (name == "SetInterface") {
        out = BusEventKind::SetInterface;
        return;
    }
    if (name == "DeviceAttached") {
        out = BusEventKind::DeviceAttached;
        return;
    }
    if (name == "DeviceDetached") {
        out = BusEventKind::DeviceDetached;
        return;
    }
    if (name == "CloneUp") {
        out = BusEventKind::CloneUp;
        return;
    }
    if (name == "CloneDown") {
        out = BusEventKind::CloneDown;
        return;
    }
    throw std::runtime_error("BusEventKind has no member " + name);
}

inline Value write(CatchEventKind value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Motion");
    case 1:
        return Value("Usages");
    case 2:
        return Value("Traffic");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, CatchEventKind& out) {
    if (!value.is_string()) {
        out = static_cast<CatchEventKind>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Motion") {
        out = CatchEventKind::Motion;
        return;
    }
    if (name == "Usages") {
        out = CatchEventKind::Usages;
        return;
    }
    if (name == "Traffic") {
        out = CatchEventKind::Traffic;
        return;
    }
    throw std::runtime_error("CatchEventKind has no member " + name);
}

inline Value write(ClockDomain value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("HostChip");
    case 1:
        return Value("DeviceChip");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, ClockDomain& out) {
    if (!value.is_string()) {
        out = static_cast<ClockDomain>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "HostChip") {
        out = ClockDomain::HostChip;
        return;
    }
    if (name == "DeviceChip") {
        out = ClockDomain::DeviceChip;
        return;
    }
    throw std::runtime_error("ClockDomain has no member " + name);
}

inline Value write(InputKind value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Press");
    case 1:
        return Value("Release");
    case 2:
        return Value("Motion");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, InputKind& out) {
    if (!value.is_string()) {
        out = static_cast<InputKind>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Press") {
        out = InputKind::Press;
        return;
    }
    if (name == "Release") {
        out = InputKind::Release;
        return;
    }
    if (name == "Motion") {
        out = InputKind::Motion;
        return;
    }
    throw std::runtime_error("InputKind has no member " + name);
}

inline Value write(LogLevel value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Error");
    case 1:
        return Value("Warn");
    case 2:
        return Value("Info");
    case 3:
        return Value("Debug");
    case 4:
        return Value("Verbose");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, LogLevel& out) {
    if (!value.is_string()) {
        out = static_cast<LogLevel>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Error") {
        out = LogLevel::Error;
        return;
    }
    if (name == "Warn") {
        out = LogLevel::Warn;
        return;
    }
    if (name == "Info") {
        out = LogLevel::Info;
        return;
    }
    if (name == "Debug") {
        out = LogLevel::Debug;
        return;
    }
    if (name == "Verbose") {
        out = LogLevel::Verbose;
        return;
    }
    throw std::runtime_error("LogLevel has no member " + name);
}

inline Value write(FrameType value) {
    switch (static_cast<std::int64_t>(value)) {
    case 1:
        return Value("Move");
    case 3:
        return Value("Inject");
    case 4:
        return Value("Reset");
    case 5:
        return Value("Query");
    case 6:
        return Value("Resp");
    case 7:
        return Value("RebootDl");
    case 8:
        return Value("Log");
    case 9:
        return Value("Led");
    case 10:
        return Value("Lock");
    case 11:
        return Value("Catch");
    case 12:
        return Value("MotionEvent");
    case 15:
        return Value("UsageEvent");
    case 22:
        return Value("TrafficEvent");
    case 17:
        return Value("Option");
    case 18:
        return Value("ClipAppend");
    case 19:
        return Value("ClipCtrl");
    case 20:
        return Value("ClipSet");
    case 21:
        return Value("ClipTrigger");
    case 23:
        return Value("Update");
    case 24:
        return Value("UpdateResp");
    case 25:
        return Value("Raw");
    case 26:
        return Value("Transfer");
    case 27:
        return Value("TransferResp");
    case 28:
        return Value("Rewrite");
    case 29:
        return Value("Patch");
    case 30:
        return Value("Transform");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, FrameType& out) {
    if (!value.is_string()) {
        out = static_cast<FrameType>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Move") {
        out = FrameType::Move;
        return;
    }
    if (name == "Inject") {
        out = FrameType::Inject;
        return;
    }
    if (name == "Reset") {
        out = FrameType::Reset;
        return;
    }
    if (name == "Query") {
        out = FrameType::Query;
        return;
    }
    if (name == "Resp") {
        out = FrameType::Resp;
        return;
    }
    if (name == "RebootDl") {
        out = FrameType::RebootDl;
        return;
    }
    if (name == "Log") {
        out = FrameType::Log;
        return;
    }
    if (name == "Led") {
        out = FrameType::Led;
        return;
    }
    if (name == "Lock") {
        out = FrameType::Lock;
        return;
    }
    if (name == "Catch") {
        out = FrameType::Catch;
        return;
    }
    if (name == "MotionEvent") {
        out = FrameType::MotionEvent;
        return;
    }
    if (name == "UsageEvent") {
        out = FrameType::UsageEvent;
        return;
    }
    if (name == "TrafficEvent") {
        out = FrameType::TrafficEvent;
        return;
    }
    if (name == "Option") {
        out = FrameType::Option;
        return;
    }
    if (name == "ClipAppend") {
        out = FrameType::ClipAppend;
        return;
    }
    if (name == "ClipCtrl") {
        out = FrameType::ClipCtrl;
        return;
    }
    if (name == "ClipSet") {
        out = FrameType::ClipSet;
        return;
    }
    if (name == "ClipTrigger") {
        out = FrameType::ClipTrigger;
        return;
    }
    if (name == "Update") {
        out = FrameType::Update;
        return;
    }
    if (name == "UpdateResp") {
        out = FrameType::UpdateResp;
        return;
    }
    if (name == "Raw") {
        out = FrameType::Raw;
        return;
    }
    if (name == "Transfer") {
        out = FrameType::Transfer;
        return;
    }
    if (name == "TransferResp") {
        out = FrameType::TransferResp;
        return;
    }
    if (name == "Rewrite") {
        out = FrameType::Rewrite;
        return;
    }
    if (name == "Patch") {
        out = FrameType::Patch;
        return;
    }
    if (name == "Transform") {
        out = FrameType::Transform;
        return;
    }
    throw std::runtime_error("FrameType has no member " + name);
}

inline Value write(Action value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("SoftRelease");
    case 1:
        return Value("Press");
    case 2:
        return Value("ForceRelease");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, Action& out) {
    if (!value.is_string()) {
        out = static_cast<Action>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "SoftRelease") {
        out = Action::SoftRelease;
        return;
    }
    if (name == "Press") {
        out = Action::Press;
        return;
    }
    if (name == "ForceRelease") {
        out = Action::ForceRelease;
        return;
    }
    throw std::runtime_error("Action has no member " + name);
}

inline Value write(Axis value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("X");
    case 1:
        return Value("Y");
    case 2:
        return Value("Wheel");
    case 3:
        return Value("Pan");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, Axis& out) {
    if (!value.is_string()) {
        out = static_cast<Axis>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "X") {
        out = Axis::X;
        return;
    }
    if (name == "Y") {
        out = Axis::Y;
        return;
    }
    if (name == "Wheel") {
        out = Axis::Wheel;
        return;
    }
    if (name == "Pan") {
        out = Axis::Pan;
        return;
    }
    throw std::runtime_error("Axis has no member " + name);
}

inline Value write(Blanket value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Aim");
    case 1:
        return Value("Wheel");
    case 2:
        return Value("Buttons");
    case 3:
        return Value("Keys");
    case 4:
        return Value("Media");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, Blanket& out) {
    if (!value.is_string()) {
        out = static_cast<Blanket>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Aim") {
        out = Blanket::Aim;
        return;
    }
    if (name == "Wheel") {
        out = Blanket::Wheel;
        return;
    }
    if (name == "Buttons") {
        out = Blanket::Buttons;
        return;
    }
    if (name == "Keys") {
        out = Blanket::Keys;
        return;
    }
    if (name == "Media") {
        out = Blanket::Media;
        return;
    }
    throw std::runtime_error("Blanket has no member " + name);
}

inline Value write(Class value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Button");
    case 1:
        return Value("Key");
    case 2:
        return Value("Media");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, Class& out) {
    if (!value.is_string()) {
        out = static_cast<Class>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Button") {
        out = Class::Button;
        return;
    }
    if (name == "Key") {
        out = Class::Key;
        return;
    }
    if (name == "Media") {
        out = Class::Media;
        return;
    }
    throw std::runtime_error("Class has no member " + name);
}

inline Value write(ClipAction value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Start");
    case 1:
        return Value("Stop");
    case 2:
        return Value("Pause");
    case 3:
        return Value("Resume");
    case 4:
        return Value("Restart");
    case 5:
        return Value("Toggle");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, ClipAction& out) {
    if (!value.is_string()) {
        out = static_cast<ClipAction>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Start") {
        out = ClipAction::Start;
        return;
    }
    if (name == "Stop") {
        out = ClipAction::Stop;
        return;
    }
    if (name == "Pause") {
        out = ClipAction::Pause;
        return;
    }
    if (name == "Resume") {
        out = ClipAction::Resume;
        return;
    }
    if (name == "Restart") {
        out = ClipAction::Restart;
        return;
    }
    if (name == "Toggle") {
        out = ClipAction::Toggle;
        return;
    }
    throw std::runtime_error("ClipAction has no member " + name);
}

inline Value write(ClipState value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Idle");
    case 1:
        return Value("Playing");
    case 2:
        return Value("Paused");
    case 3:
        return Value("Faulted");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, ClipState& out) {
    if (!value.is_string()) {
        out = static_cast<ClipState>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Idle") {
        out = ClipState::Idle;
        return;
    }
    if (name == "Playing") {
        out = ClipState::Playing;
        return;
    }
    if (name == "Paused") {
        out = ClipState::Paused;
        return;
    }
    if (name == "Faulted") {
        out = ClipState::Faulted;
        return;
    }
    throw std::runtime_error("ClipState has no member " + name);
}

inline Value write(DeviceKind value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Unknown");
    case 1:
        return Value("Keyboard");
    case 2:
        return Value("Mouse");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, DeviceKind& out) {
    if (!value.is_string()) {
        out = static_cast<DeviceKind>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Unknown") {
        out = DeviceKind::Unknown;
        return;
    }
    if (name == "Keyboard") {
        out = DeviceKind::Keyboard;
        return;
    }
    if (name == "Mouse") {
        out = DeviceKind::Mouse;
        return;
    }
    throw std::runtime_error("DeviceKind has no member " + name);
}

inline Value write(Direction value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Both");
    case 1:
        return Value("Positive");
    case 2:
        return Value("Negative");
    case 3:
        return Value("With");
    case 4:
        return Value("Against");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, Direction& out) {
    if (!value.is_string()) {
        out = static_cast<Direction>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Both") {
        out = Direction::Both;
        return;
    }
    if (name == "Positive") {
        out = Direction::Positive;
        return;
    }
    if (name == "Negative") {
        out = Direction::Negative;
        return;
    }
    if (name == "With") {
        out = Direction::With;
        return;
    }
    if (name == "Against") {
        out = Direction::Against;
        return;
    }
    if (name == "Press") {
        out = Direction::Press;
        return;
    }
    if (name == "Release") {
        out = Direction::Release;
        return;
    }
    if (name == "In") {
        out = Direction::In;
        return;
    }
    if (name == "Out") {
        out = Direction::Out;
        return;
    }
    throw std::runtime_error("Direction has no member " + name);
}

inline Value write(Edge value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Both");
    case 1:
        return Value("Press");
    case 2:
        return Value("Release");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, Edge& out) {
    if (!value.is_string()) {
        out = static_cast<Edge>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Both") {
        out = Edge::Both;
        return;
    }
    if (name == "Press") {
        out = Edge::Press;
        return;
    }
    if (name == "Release") {
        out = Edge::Release;
        return;
    }
    throw std::runtime_error("Edge has no member " + name);
}

inline Value write(LedMode value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Auto");
    case 1:
        return Value("Off");
    case 2:
        return Value("Solid");
    case 3:
        return Value("Blink");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, LedMode& out) {
    if (!value.is_string()) {
        out = static_cast<LedMode>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Auto") {
        out = LedMode::Auto;
        return;
    }
    if (name == "Off") {
        out = LedMode::Off;
        return;
    }
    if (name == "Solid") {
        out = LedMode::Solid;
        return;
    }
    if (name == "Blink") {
        out = LedMode::Blink;
        return;
    }
    throw std::runtime_error("LedMode has no member " + name);
}

inline Value write(LedTarget value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Device");
    case 1:
        return Value("Host");
    case 2:
        return Value("Both");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, LedTarget& out) {
    if (!value.is_string()) {
        out = static_cast<LedTarget>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Device") {
        out = LedTarget::Device;
        return;
    }
    if (name == "Host") {
        out = LedTarget::Host;
        return;
    }
    if (name == "Both") {
        out = LedTarget::Both;
        return;
    }
    throw std::runtime_error("LedTarget has no member " + name);
}

inline Value write(LockTargetKind value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("X");
    case 1:
        return Value("Y");
    case 2:
        return Value("Wheel");
    case 3:
        return Value("Pan");
    case 4:
        return Value("Usage");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, LockTargetKind& out) {
    if (!value.is_string()) {
        out = static_cast<LockTargetKind>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "X") {
        out = LockTargetKind::X;
        return;
    }
    if (name == "Y") {
        out = LockTargetKind::Y;
        return;
    }
    if (name == "Wheel") {
        out = LockTargetKind::Wheel;
        return;
    }
    if (name == "Pan") {
        out = LockTargetKind::Pan;
        return;
    }
    if (name == "Usage") {
        out = LockTargetKind::Usage;
        return;
    }
    throw std::runtime_error("LockTargetKind has no member " + name);
}

inline Value write(MotionKind value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Cursor");
    case 1:
        return Value("Wheel");
    case 2:
        return Value("Pan");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, MotionKind& out) {
    if (!value.is_string()) {
        out = static_cast<MotionKind>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Cursor") {
        out = MotionKind::Cursor;
        return;
    }
    if (name == "Wheel") {
        out = MotionKind::Wheel;
        return;
    }
    if (name == "Pan") {
        out = MotionKind::Pan;
        return;
    }
    throw std::runtime_error("MotionKind has no member " + name);
}

inline Value write(MoveTiming value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Ride");
    case 1:
        return Value("Now");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, MoveTiming& out) {
    if (!value.is_string()) {
        out = static_cast<MoveTiming>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Ride") {
        out = MoveTiming::Ride;
        return;
    }
    if (name == "Now") {
        out = MoveTiming::Now;
        return;
    }
    throw std::runtime_error("MoveTiming has no member " + name);
}

inline Value write(PatchSection value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Device");
    case 1:
        return Value("Config");
    case 2:
        return Value("Report");
    case 3:
        return Value("String");
    case 4:
        return Value("Bos");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, PatchSection& out) {
    if (!value.is_string()) {
        out = static_cast<PatchSection>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Device") {
        out = PatchSection::Device;
        return;
    }
    if (name == "Config") {
        out = PatchSection::Config;
        return;
    }
    if (name == "Report") {
        out = PatchSection::Report;
        return;
    }
    if (name == "String") {
        out = PatchSection::String;
        return;
    }
    if (name == "Bos") {
        out = PatchSection::Bos;
        return;
    }
    throw std::runtime_error("PatchSection has no member " + name);
}

inline Value write(PendingMotion value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Keep");
    case 1:
        return Value("Flush");
    case 2:
        return Value("Discard");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, PendingMotion& out) {
    if (!value.is_string()) {
        out = static_cast<PendingMotion>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Keep") {
        out = PendingMotion::Keep;
        return;
    }
    if (name == "Flush") {
        out = PendingMotion::Flush;
        return;
    }
    if (name == "Discard") {
        out = PendingMotion::Discard;
        return;
    }
    throw std::runtime_error("PendingMotion has no member " + name);
}

inline Value write(RebootTarget value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("DeviceDownload");
    case 1:
        return Value("HostDownload");
    case 2:
        return Value("DeviceRun");
    case 3:
        return Value("HostRun");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, RebootTarget& out) {
    if (!value.is_string()) {
        out = static_cast<RebootTarget>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "DeviceDownload") {
        out = RebootTarget::DeviceDownload;
        return;
    }
    if (name == "HostDownload") {
        out = RebootTarget::HostDownload;
        return;
    }
    if (name == "DeviceRun") {
        out = RebootTarget::DeviceRun;
        return;
    }
    if (name == "HostRun") {
        out = RebootTarget::HostRun;
        return;
    }
    throw std::runtime_error("RebootTarget has no member " + name);
}

inline Value write(RewriteAction value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Pass");
    case 1:
        return Value("Drop");
    case 2:
        return Value("Patch");
    case 3:
        return Value("Replace");
    case 4:
        return Value("Answer");
    case 5:
        return Value("Stall");
    case 6:
        return Value("Nak");
    case 7:
        return Value("ReplyPatch");
    case 8:
        return Value("ReplyReplace");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, RewriteAction& out) {
    if (!value.is_string()) {
        out = static_cast<RewriteAction>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Pass") {
        out = RewriteAction::Pass;
        return;
    }
    if (name == "Drop") {
        out = RewriteAction::Drop;
        return;
    }
    if (name == "Patch") {
        out = RewriteAction::Patch;
        return;
    }
    if (name == "Replace") {
        out = RewriteAction::Replace;
        return;
    }
    if (name == "Answer") {
        out = RewriteAction::Answer;
        return;
    }
    if (name == "Stall") {
        out = RewriteAction::Stall;
        return;
    }
    if (name == "Nak") {
        out = RewriteAction::Nak;
        return;
    }
    if (name == "ReplyPatch") {
        out = RewriteAction::ReplyPatch;
        return;
    }
    if (name == "ReplyReplace") {
        out = RewriteAction::ReplyReplace;
        return;
    }
    throw std::runtime_error("RewriteAction has no member " + name);
}

inline Value write(RewriteClass value) {
    switch (static_cast<std::int64_t>(value)) {
    case 4:
        return Value("HidIn");
    case 5:
        return Value("HidOut");
    case 6:
        return Value("VendorInterrupt");
    case 7:
        return Value("VendorBulk");
    case 8:
        return Value("Control");
    case 9:
        return Value("Emit");
    case 255:
        return Value("Any");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, RewriteClass& out) {
    if (!value.is_string()) {
        out = static_cast<RewriteClass>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "HidIn") {
        out = RewriteClass::HidIn;
        return;
    }
    if (name == "HidOut") {
        out = RewriteClass::HidOut;
        return;
    }
    if (name == "VendorInterrupt") {
        out = RewriteClass::VendorInterrupt;
        return;
    }
    if (name == "VendorBulk") {
        out = RewriteClass::VendorBulk;
        return;
    }
    if (name == "Control") {
        out = RewriteClass::Control;
        return;
    }
    if (name == "Emit") {
        out = RewriteClass::Emit;
        return;
    }
    if (name == "Any") {
        out = RewriteClass::Any;
        return;
    }
    throw std::runtime_error("RewriteClass has no member " + name);
}

inline Value write(TransferStatus value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Ok");
    case 252:
        return Value("Refused");
    case 253:
        return Value("Stall");
    case 254:
        return Value("Nak");
    case 255:
        return Value("NoDevice");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, TransferStatus& out) {
    if (!value.is_string()) {
        out = static_cast<TransferStatus>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Ok") {
        out = TransferStatus::Ok;
        return;
    }
    if (name == "Refused") {
        out = TransferStatus::Refused;
        return;
    }
    if (name == "Stall") {
        out = TransferStatus::Stall;
        return;
    }
    if (name == "Nak") {
        out = TransferStatus::Nak;
        return;
    }
    if (name == "NoDevice") {
        out = TransferStatus::NoDevice;
        return;
    }
    throw std::runtime_error("TransferStatus has no member " + name);
}

inline Value write(TransformOp value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Remap");
    case 1:
        return Value("Swap");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, TransformOp& out) {
    if (!value.is_string()) {
        out = static_cast<TransformOp>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Remap") {
        out = TransformOp::Remap;
        return;
    }
    if (name == "Swap") {
        out = TransformOp::Swap;
        return;
    }
    throw std::runtime_error("TransformOp has no member " + name);
}

inline Value write(CatchClass value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Button");
    case 1:
        return Value("Key");
    case 2:
        return Value("Media");
    case 3:
        return Value("Axis");
    case 4:
        return Value("HidIn");
    case 5:
        return Value("HidOut");
    case 6:
        return Value("VendorInterrupt");
    case 7:
        return Value("VendorBulk");
    case 8:
        return Value("Control");
    case 9:
        return Value("Emit");
    case 10:
        return Value("Bus");
    case 11:
        return Value("ClipTransfer");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, CatchClass& out) {
    if (!value.is_string()) {
        out = static_cast<CatchClass>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Button") {
        out = CatchClass::Button;
        return;
    }
    if (name == "Key") {
        out = CatchClass::Key;
        return;
    }
    if (name == "Media") {
        out = CatchClass::Media;
        return;
    }
    if (name == "Axis") {
        out = CatchClass::Axis;
        return;
    }
    if (name == "HidIn") {
        out = CatchClass::HidIn;
        return;
    }
    if (name == "HidOut") {
        out = CatchClass::HidOut;
        return;
    }
    if (name == "VendorInterrupt") {
        out = CatchClass::VendorInterrupt;
        return;
    }
    if (name == "VendorBulk") {
        out = CatchClass::VendorBulk;
        return;
    }
    if (name == "Control") {
        out = CatchClass::Control;
        return;
    }
    if (name == "Emit") {
        out = CatchClass::Emit;
        return;
    }
    if (name == "Bus") {
        out = CatchClass::Bus;
        return;
    }
    if (name == "ClipTransfer") {
        out = CatchClass::ClipTransfer;
        return;
    }
    throw std::runtime_error("CatchClass has no member " + name);
}

inline Value write(TrafficClass value) {
    switch (static_cast<std::int64_t>(value)) {
    case 4:
        return Value("HidIn");
    case 5:
        return Value("HidOut");
    case 6:
        return Value("VendorInterrupt");
    case 7:
        return Value("VendorBulk");
    case 8:
        return Value("Control");
    case 9:
        return Value("Emit");
    case 10:
        return Value("Bus");
    case 11:
        return Value("ClipTransfer");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, TrafficClass& out) {
    if (!value.is_string()) {
        out = static_cast<TrafficClass>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "HidIn") {
        out = TrafficClass::HidIn;
        return;
    }
    if (name == "HidOut") {
        out = TrafficClass::HidOut;
        return;
    }
    if (name == "VendorInterrupt") {
        out = TrafficClass::VendorInterrupt;
        return;
    }
    if (name == "VendorBulk") {
        out = TrafficClass::VendorBulk;
        return;
    }
    if (name == "Control") {
        out = TrafficClass::Control;
        return;
    }
    if (name == "Emit") {
        out = TrafficClass::Emit;
        return;
    }
    if (name == "Bus") {
        out = TrafficClass::Bus;
        return;
    }
    if (name == "ClipTransfer") {
        out = TrafficClass::ClipTransfer;
        return;
    }
    throw std::runtime_error("TrafficClass has no member " + name);
}

inline Value write(ImageState value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("New");
    case 1:
        return Value("PendingVerify");
    case 2:
        return Value("Valid");
    case 3:
        return Value("Invalid");
    case 4:
        return Value("Aborted");
    case 255:
        return Value("Unknown");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, ImageState& out) {
    if (!value.is_string()) {
        out = static_cast<ImageState>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "New") {
        out = ImageState::New;
        return;
    }
    if (name == "PendingVerify") {
        out = ImageState::PendingVerify;
        return;
    }
    if (name == "Valid") {
        out = ImageState::Valid;
        return;
    }
    if (name == "Invalid") {
        out = ImageState::Invalid;
        return;
    }
    if (name == "Aborted") {
        out = ImageState::Aborted;
        return;
    }
    if (name == "Unknown") {
        out = ImageState::Unknown;
        return;
    }
    throw std::runtime_error("ImageState has no member " + name);
}

inline Value write(UpdateTarget value) {
    switch (static_cast<std::int64_t>(value)) {
    case 0:
        return Value("Device");
    case 1:
        return Value("Host");
    default:
        return Value(static_cast<std::int64_t>(value));
    }
}

inline void read(const Value& value, UpdateTarget& out) {
    if (!value.is_string()) {
        out = static_cast<UpdateTarget>(value.as_int());
        return;
    }
    const auto& name = value.as_string();
    if (name == "Device") {
        out = UpdateTarget::Device;
        return;
    }
    if (name == "Host") {
        out = UpdateTarget::Host;
        return;
    }
    throw std::runtime_error("UpdateTarget has no member " + name);
}

inline Value write(Button value) { return Value(static_cast<std::int64_t>(value.value)); }
inline void read(const Value& value, Button& out) { out.value = static_cast<decltype(out.value)>(value.as_int()); }

inline Value write(Key value) { return Value(static_cast<std::int64_t>(value.value)); }
inline void read(const Value& value, Key& out) { out.value = static_cast<decltype(out.value)>(value.as_int()); }

inline Value write(MediaKey value) { return Value(static_cast<std::int64_t>(value.value)); }
inline void read(const Value& value, MediaKey& out) { out.value = static_cast<decltype(out.value)>(value.as_int()); }

Value write(const Setup& value);
void read(const Value& value, Setup& out);
Value write(const ClipTrigger& value);
void read(const Value& value, ClipTrigger& out);
Value write(const ClipPacketTrigger& value);
void read(const Value& value, ClipPacketTrigger& out);
Value write(const ClipStatus& value);
void read(const Value& value, ClipStatus& out);
Value write(const PortInfo& value);
void read(const Value& value, PortInfo& out);
Value write(const Version& value);
void read(const Value& value, Version& out);
Value write(const DeviceInfo& value);
void read(const Value& value, DeviceInfo& out);
Value write(const BoxInfo& value);
void read(const Value& value, BoxInfo& out);
Value write(const TransferOutcome& value);
void read(const Value& value, TransferOutcome& out);
Value write(const RewriteRule& value);
void read(const Value& value, RewriteRule& out);
Value write(const RewriteEntry& value);
void read(const Value& value, RewriteEntry& out);
Value write(const RewriteTable& value);
void read(const Value& value, RewriteTable& out);
Value write(const Patch& value);
void read(const Value& value, Patch& out);
Value write(const PatchEntry& value);
void read(const Value& value, PatchEntry& out);
Value write(const PatchSet& value);
void read(const Value& value, PatchSet& out);
Value write(const Transform& value);
void read(const Value& value, Transform& out);
Value write(const Transforms& value);
void read(const Value& value, Transforms& out);
Value write(const ChipFirmware& value);
void read(const Value& value, ChipFirmware& out);
Value write(const FirmwareInfo& value);
void read(const Value& value, FirmwareInfo& out);
Value write(const Health& value);
void read(const Value& value, Health& out);
Value write(const MouseCaps& value);
void read(const Value& value, MouseCaps& out);
Value write(const KbdCaps& value);
void read(const Value& value, KbdCaps& out);
Value write(const Caps& value);
void read(const Value& value, Caps& out);
Value write(const Rate& value);
void read(const Value& value, Rate& out);
Value write(const Stats& value);
void read(const Value& value, Stats& out);
Value write(const LockEntry& value);
void read(const Value& value, LockEntry& out);
Value write(const Locks& value);
void read(const Value& value, Locks& out);
Value write(const ClockEstimate& value);
void read(const Value& value, ClockEstimate& out);
Value write(const CatchEntry& value);
void read(const Value& value, CatchEntry& out);
Value write(const CatchState& value);
void read(const Value& value, CatchState& out);
Value write(const ImperfectStatus& value);
void read(const Value& value, ImperfectStatus& out);
Value write(const Bearing& value);
void read(const Value& value, Bearing& out);
Value write(const RenderStatus& value);
void read(const Value& value, RenderStatus& out);
Value write(const SpreadStatus& value);
void read(const Value& value, SpreadStatus& out);
Value write(const CountersSnapshot& value);
void read(const Value& value, CountersSnapshot& out);
Value write(const UsageSnapshot& value);
void read(const Value& value, UsageSnapshot& out);
Value write(const TrafficEvent& value);
void read(const Value& value, TrafficEvent& out);
Value write(const BusEvent& value);
void read(const Value& value, BusEvent& out);
Value write(const MotionEvent& value);
void read(const Value& value, MotionEvent& out);
Value write(const Stamped& value);
void read(const Value& value, Stamped& out);
Value write(const LogLine& value);
void read(const Value& value, LogLine& out);

inline Value write(const Setup& value) {
    Object out;
    out.push_back({"request_type", write(value.request_type)});
    out.push_back({"request", write(value.request)});
    out.push_back({"value", write(value.value)});
    out.push_back({"index", write(value.index)});
    out.push_back({"length", write(value.length)});
    return Value(std::move(out));
}

inline void read(const Value& value, Setup& out) {
    read(value["request_type"], out.request_type);
    read(value["request"], out.request);
    read(value["value"], out.value);
    read(value["index"], out.index);
    read(value["length"], out.length);
}

inline Value write(const ClipTrigger& value) {
    Object out;
    out.push_back({"on", write(value.on)});
    out.push_back({"edge", write(value.edge)});
    out.push_back({"action", write(value.action)});
    out.push_back({"consume", write(value.consume)});
    return Value(std::move(out));
}

inline void read(const Value& value, ClipTrigger& out) {
    read(value["on"], out.on);
    read(value["edge"], out.edge);
    read(value["action"], out.action);
    read(value["consume"], out.consume);
}

inline Value write(const ClipPacketTrigger& value) {
    Object out;
    out.push_back({"class_", write(value.class_)});
    out.push_back({"id", write(value.id)});
    out.push_back({"direction", write(value.direction)});
    out.push_back({"action", write(value.action)});
    out.push_back({"consume", write(value.consume)});
    out.push_back({"once_per_run", write(value.once_per_run)});
    out.push_back({"selector_len", write(value.selector_len)});
    out.push_back({"match_bytes", write(value.match_bytes)});
    out.push_back({"mask", write(value.mask)});
    out.push_back({"hits", write(value.hits)});
    return Value(std::move(out));
}

inline void read(const Value& value, ClipPacketTrigger& out) {
    read(value["class_"], out.class_);
    read(value["id"], out.id);
    read(value["direction"], out.direction);
    read(value["action"], out.action);
    read(value["consume"], out.consume);
    read(value["once_per_run"], out.once_per_run);
    read(value["selector_len"], out.selector_len);
    read(value["match_bytes"], out.match_bytes);
    read(value["mask"], out.mask);
    read(value["hits"], out.hits);
}

inline Value write(const ClipStatus& value) {
    Object out;
    out.push_back({"state", write(value.state)});
    out.push_back({"free", write(value.free)});
    out.push_back({"total", write(value.total)});
    out.push_back({"played", write(value.played)});
    out.push_back({"ticks", write(value.ticks)});
    out.push_back({"underruns", write(value.underruns)});
    out.push_back({"overruns", write(value.overruns)});
    out.push_back({"seq_gaps", write(value.seq_gaps)});
    out.push_back({"xfers", write(value.xfers)});
    out.push_back({"xfer_errs", write(value.xfer_errs)});
    out.push_back({"gated", write(value.gated)});
    out.push_back({"held", write(value.held)});
    return Value(std::move(out));
}

inline void read(const Value& value, ClipStatus& out) {
    read(value["state"], out.state);
    read(value["free"], out.free);
    read(value["total"], out.total);
    read(value["played"], out.played);
    read(value["ticks"], out.ticks);
    read(value["underruns"], out.underruns);
    read(value["overruns"], out.overruns);
    read(value["seq_gaps"], out.seq_gaps);
    read(value["xfers"], out.xfers);
    read(value["xfer_errs"], out.xfer_errs);
    read(value["gated"], out.gated);
    read(value["held"], out.held);
}

inline Value write(const PortInfo& value) {
    Object out;
    out.push_back({"path", write(value.path)});
    out.push_back({"vid", write(value.vid)});
    out.push_back({"pid", write(value.pid)});
    out.push_back({"serial", write(value.serial)});
    return Value(std::move(out));
}

inline void read(const Value& value, PortInfo& out) {
    read(value["path"], out.path);
    read(value["vid"], out.vid);
    read(value["pid"], out.pid);
    read(value["serial"], out.serial);
}

inline Value write(const Version& value) {
    Object out;
    out.push_back({"proto_ver", write(value.proto_ver)});
    out.push_back({"fw_major", write(value.fw_major)});
    out.push_back({"fw_minor", write(value.fw_minor)});
    out.push_back({"fw_patch", write(value.fw_patch)});
    out.push_back({"mac", write(value.mac)});
    out.push_back({"name", write(value.name)});
    return Value(std::move(out));
}

inline void read(const Value& value, Version& out) {
    read(value["proto_ver"], out.proto_ver);
    read(value["fw_major"], out.fw_major);
    read(value["fw_minor"], out.fw_minor);
    read(value["fw_patch"], out.fw_patch);
    read(value["mac"], out.mac);
    read(value["name"], out.name);
}

inline Value write(const DeviceInfo& value) {
    Object out;
    out.push_back({"vid", write(value.vid)});
    out.push_back({"pid", write(value.pid)});
    out.push_back({"bcd_device", write(value.bcd_device)});
    out.push_back({"bcd_usb", write(value.bcd_usb)});
    out.push_back({"has_serial", write(value.has_serial)});
    out.push_back({"has_bos", write(value.has_bos)});
    out.push_back({"kind", write(value.kind)});
    out.push_back({"product", write(value.product)});
    return Value(std::move(out));
}

inline void read(const Value& value, DeviceInfo& out) {
    read(value["vid"], out.vid);
    read(value["pid"], out.pid);
    read(value["bcd_device"], out.bcd_device);
    read(value["bcd_usb"], out.bcd_usb);
    read(value["has_serial"], out.has_serial);
    read(value["has_bos"], out.has_bos);
    read(value["kind"], out.kind);
    read(value["product"], out.product);
}

inline Value write(const BoxInfo& value) {
    Object out;
    out.push_back({"port", write(value.port)});
    out.push_back({"version", write(value.version)});
    out.push_back({"device", write(value.device)});
    return Value(std::move(out));
}

inline void read(const Value& value, BoxInfo& out) {
    read(value["port"], out.port);
    read(value["version"], out.version);
    read(value["device"], out.device);
}

inline Value write(const TransferOutcome& value) {
    Object out;
    out.push_back({"status", write(value.status)});
    out.push_back({"data", write(value.data)});
    return Value(std::move(out));
}

inline void read(const Value& value, TransferOutcome& out) {
    read(value["status"], out.status);
    read(value["data"], out.data);
}

inline Value write(const RewriteRule& value) {
    Object out;
    out.push_back({"class_", write(value.class_)});
    out.push_back({"id", write(value.id)});
    out.push_back({"direction", write(value.direction)});
    out.push_back({"action", write(value.action)});
    out.push_back({"offset", write(value.offset)});
    out.push_back({"match_bytes", write(value.match_bytes)});
    out.push_back({"mask", write(value.mask)});
    out.push_back({"payload", write(value.payload)});
    return Value(std::move(out));
}

inline void read(const Value& value, RewriteRule& out) {
    read(value["class_"], out.class_);
    read(value["id"], out.id);
    read(value["direction"], out.direction);
    read(value["action"], out.action);
    read(value["offset"], out.offset);
    read(value["match_bytes"], out.match_bytes);
    read(value["mask"], out.mask);
    read(value["payload"], out.payload);
}

inline Value write(const RewriteEntry& value) {
    Object out;
    out.push_back({"class_", write(value.class_)});
    out.push_back({"id", write(value.id)});
    out.push_back({"direction", write(value.direction)});
    out.push_back({"action", write(value.action)});
    out.push_back({"match_len", write(value.match_len)});
    out.push_back({"offset", write(value.offset)});
    out.push_back({"payload_len", write(value.payload_len)});
    out.push_back({"hits", write(value.hits)});
    return Value(std::move(out));
}

inline void read(const Value& value, RewriteEntry& out) {
    read(value["class_"], out.class_);
    read(value["id"], out.id);
    read(value["direction"], out.direction);
    read(value["action"], out.action);
    read(value["match_len"], out.match_len);
    read(value["offset"], out.offset);
    read(value["payload_len"], out.payload_len);
    read(value["hits"], out.hits);
}

inline Value write(const RewriteTable& value) {
    Object out;
    out.push_back({"table_full", write(value.table_full)});
    out.push_back({"generation", write(value.generation)});
    out.push_back({"entries", write(value.entries)});
    return Value(std::move(out));
}

inline void read(const Value& value, RewriteTable& out) {
    read(value["table_full"], out.table_full);
    read(value["generation"], out.generation);
    read(value["entries"], out.entries);
}

inline Value write(const Patch& value) {
    Object out;
    out.push_back({"section", write(value.section)});
    out.push_back({"cfg", write(value.cfg)});
    out.push_back({"index", write(value.index)});
    out.push_back({"offset", write(value.offset)});
    out.push_back({"bytes", write(value.bytes)});
    return Value(std::move(out));
}

inline void read(const Value& value, Patch& out) {
    read(value["section"], out.section);
    read(value["cfg"], out.cfg);
    read(value["index"], out.index);
    read(value["offset"], out.offset);
    read(value["bytes"], out.bytes);
}

inline Value write(const PatchEntry& value) {
    Object out;
    out.push_back({"section", write(value.section)});
    out.push_back({"cfg", write(value.cfg)});
    out.push_back({"index", write(value.index)});
    out.push_back({"offset", write(value.offset)});
    out.push_back({"len", write(value.len)});
    return Value(std::move(out));
}

inline void read(const Value& value, PatchEntry& out) {
    read(value["section"], out.section);
    read(value["cfg"], out.cfg);
    read(value["index"], out.index);
    read(value["offset"], out.offset);
    read(value["len"], out.len);
}

inline Value write(const PatchSet& value) {
    Object out;
    out.push_back({"applied", write(value.applied)});
    out.push_back({"pending", write(value.pending)});
    out.push_back({"refused", write(value.refused)});
    out.push_back({"table_full", write(value.table_full)});
    out.push_back({"entries", write(value.entries)});
    return Value(std::move(out));
}

inline void read(const Value& value, PatchSet& out) {
    read(value["applied"], out.applied);
    read(value["pending"], out.pending);
    read(value["refused"], out.refused);
    read(value["table_full"], out.table_full);
    read(value["entries"], out.entries);
}

inline Value write(const Transform& value) {
    Object out;
    out.push_back({"op", write(value.op)});
    out.push_back({"source", write(value.source)});
    out.push_back({"dest", write(value.dest)});
    return Value(std::move(out));
}

inline void read(const Value& value, Transform& out) {
    read(value["op"], out.op);
    read(value["source"], out.source);
    read(value["dest"], out.dest);
}

inline Value write(const Transforms& value) {
    Object out;
    out.push_back({"table_full", write(value.table_full)});
    out.push_back({"entries", write(value.entries)});
    return Value(std::move(out));
}

inline void read(const Value& value, Transforms& out) {
    read(value["table_full"], out.table_full);
    read(value["entries"], out.entries);
}

inline Value write(const ChipFirmware& value) {
    Object out;
    out.push_back({"major", write(value.major)});
    out.push_back({"minor", write(value.minor)});
    out.push_back({"patch", write(value.patch)});
    out.push_back({"slot", write(value.slot)});
    out.push_back({"state", write(value.state)});
    return Value(std::move(out));
}

inline void read(const Value& value, ChipFirmware& out) {
    read(value["major"], out.major);
    read(value["minor"], out.minor);
    read(value["patch"], out.patch);
    read(value["slot"], out.slot);
    read(value["state"], out.state);
}

inline Value write(const FirmwareInfo& value) {
    Object out;
    out.push_back({"device", write(value.device)});
    out.push_back({"host", write(value.host)});
    out.push_back({"slot_size", write(value.slot_size)});
    out.push_back({"device_staged", write(value.device_staged)});
    out.push_back({"host_staged", write(value.host_staged)});
    return Value(std::move(out));
}

inline void read(const Value& value, FirmwareInfo& out) {
    read(value["device"], out.device);
    read(value["host"], out.host);
    read(value["slot_size"], out.slot_size);
    read(value["device_staged"], out.device_staged);
    read(value["host_staged"], out.host_staged);
}

inline Value write(const Health& value) {
    Object out;
    out.push_back({"link_up", write(value.link_up)});
    out.push_back({"mouse_attached", write(value.mouse_attached)});
    out.push_back({"clone_configured", write(value.clone_configured)});
    out.push_back({"injection_active", write(value.injection_active)});
    out.push_back({"rate_confident", write(value.rate_confident)});
    out.push_back({"lock_on", write(value.lock_on)});
    out.push_back({"catch_on", write(value.catch_on)});
    out.push_back({"kbd_attached", write(value.kbd_attached)});
    out.push_back({"rewrite_on", write(value.rewrite_on)});
    out.push_back({"patch_on", write(value.patch_on)});
    out.push_back({"transform_on", write(value.transform_on)});
    return Value(std::move(out));
}

inline void read(const Value& value, Health& out) {
    read(value["link_up"], out.link_up);
    read(value["mouse_attached"], out.mouse_attached);
    read(value["clone_configured"], out.clone_configured);
    read(value["injection_active"], out.injection_active);
    read(value["rate_confident"], out.rate_confident);
    read(value["lock_on"], out.lock_on);
    read(value["catch_on"], out.catch_on);
    read(value["kbd_attached"], out.kbd_attached);
    read(value["rewrite_on"], out.rewrite_on);
    read(value["patch_on"], out.patch_on);
    read(value["transform_on"], out.transform_on);
}

inline Value write(const MouseCaps& value) {
    Object out;
    out.push_back({"n_buttons", write(value.n_buttons)});
    out.push_back({"has_x", write(value.has_x)});
    out.push_back({"has_y", write(value.has_y)});
    out.push_back({"has_wheel", write(value.has_wheel)});
    out.push_back({"pan", write(value.pan)});
    out.push_back({"has_report_id", write(value.has_report_id)});
    out.push_back({"n_hid", write(value.n_hid)});
    return Value(std::move(out));
}

inline void read(const Value& value, MouseCaps& out) {
    read(value["n_buttons"], out.n_buttons);
    read(value["has_x"], out.has_x);
    read(value["has_y"], out.has_y);
    read(value["has_wheel"], out.has_wheel);
    read(value["pan"], out.pan);
    read(value["has_report_id"], out.has_report_id);
    read(value["n_hid"], out.n_hid);
}

inline Value write(const KbdCaps& value) {
    Object out;
    out.push_back({"n_keys", write(value.n_keys)});
    out.push_back({"nkro", write(value.nkro)});
    out.push_back({"has_consumer", write(value.has_consumer)});
    out.push_back({"has_system", write(value.has_system)});
    out.push_back({"has_report_id", write(value.has_report_id)});
    return Value(std::move(out));
}

inline void read(const Value& value, KbdCaps& out) {
    read(value["n_keys"], out.n_keys);
    read(value["nkro"], out.nkro);
    read(value["has_consumer"], out.has_consumer);
    read(value["has_system"], out.has_system);
    read(value["has_report_id"], out.has_report_id);
}

inline Value write(const Caps& value) {
    Object out;
    out.push_back({"mouse", write(value.mouse)});
    out.push_back({"keyboard", write(value.keyboard)});
    out.push_back({"mouse_change_driven", write(value.mouse_change_driven)});
    out.push_back({"kbd_change_driven", write(value.kbd_change_driven)});
    return Value(std::move(out));
}

inline void read(const Value& value, Caps& out) {
    read(value["mouse"], out.mouse);
    read(value["keyboard"], out.keyboard);
    read(value["mouse_change_driven"], out.mouse_change_driven);
    read(value["kbd_change_driven"], out.kbd_change_driven);
}

inline Value write(const Rate& value) {
    Object out;
    out.push_back({"native_period_us", write(value.native_period_us)});
    out.push_back({"poll_period_us", write(value.poll_period_us)});
    out.push_back({"confident", write(value.confident)});
    out.push_back({"change_driven", write(value.change_driven)});
    return Value(std::move(out));
}

inline void read(const Value& value, Rate& out) {
    read(value["native_period_us"], out.native_period_us);
    read(value["poll_period_us"], out.poll_period_us);
    read(value["confident"], out.confident);
    read(value["change_driven"], out.change_driven);
}

inline Value write(const Stats& value) {
    Object out;
    out.push_back({"inject_emits", write(value.inject_emits)});
    out.push_back({"tx_drops", write(value.tx_drops)});
    out.push_back({"tx_merges", write(value.tx_merges)});
    out.push_back({"tx_maxdepth", write(value.tx_maxdepth)});
    out.push_back({"tx_wedges", write(value.tx_wedges)});
    out.push_back({"wakeups", write(value.wakeups)});
    out.push_back({"reset_count", write(value.reset_count)});
    out.push_back({"config_count", write(value.config_count)});
    out.push_back({"link_rx_drops", write(value.link_rx_drops)});
    out.push_back({"host_rx_drops", write(value.host_rx_drops)});
    out.push_back({"relay_drops", write(value.relay_drops)});
    out.push_back({"session", write(value.session)});
    return Value(std::move(out));
}

inline void read(const Value& value, Stats& out) {
    read(value["inject_emits"], out.inject_emits);
    read(value["tx_drops"], out.tx_drops);
    read(value["tx_merges"], out.tx_merges);
    read(value["tx_maxdepth"], out.tx_maxdepth);
    read(value["tx_wedges"], out.tx_wedges);
    read(value["wakeups"], out.wakeups);
    read(value["reset_count"], out.reset_count);
    read(value["config_count"], out.config_count);
    read(value["link_rx_drops"], out.link_rx_drops);
    read(value["host_rx_drops"], out.host_rx_drops);
    read(value["relay_drops"], out.relay_drops);
    read(value["session"], out.session);
}

inline Value write(const LockEntry& value) {
    Object out;
    out.push_back({"target", write(value.target)});
    out.push_back({"is_blanket", write(value.is_blanket)});
    out.push_back({"direction", write(value.direction)});
    out.push_back({"scale", write(value.scale)});
    return Value(std::move(out));
}

inline void read(const Value& value, LockEntry& out) {
    read(value["target"], out.target);
    read(value["is_blanket"], out.is_blanket);
    read(value["direction"], out.direction);
    read(value["scale"], out.scale);
}

inline Value write(const Locks& value) {
    Object out;
    out.push_back({"entries", write(value.entries)});
    return Value(std::move(out));
}

inline void read(const Value& value, Locks& out) {
    read(value["entries"], out.entries);
}

inline Value write(const ClockEstimate& value) {
    Object out;
    out.push_back({"offset_us", write(value.offset_us)});
    out.push_back({"rate_ppb", write(value.rate_ppb)});
    out.push_back({"delay_us", write(value.delay_us)});
    out.push_back({"age_ms", write(value.age_ms)});
    return Value(std::move(out));
}

inline void read(const Value& value, ClockEstimate& out) {
    read(value["offset_us"], out.offset_us);
    read(value["rate_ppb"], out.rate_ppb);
    read(value["delay_us"], out.delay_us);
    read(value["age_ms"], out.age_ms);
}

inline Value write(const CatchEntry& value) {
    Object out;
    out.push_back({"filter", write(value.filter)});
    out.push_back({"dropped", write(value.dropped)});
    return Value(std::move(out));
}

inline void read(const Value& value, CatchEntry& out) {
    read(value["filter"], out.filter);
    read(value["dropped"], out.dropped);
}

inline Value write(const CatchState& value) {
    Object out;
    out.push_back({"table_full", write(value.table_full)});
    out.push_back({"dropped", write(value.dropped)});
    out.push_back({"clock", write(value.clock)});
    out.push_back({"entries", write(value.entries)});
    return Value(std::move(out));
}

inline void read(const Value& value, CatchState& out) {
    read(value["table_full"], out.table_full);
    read(value["dropped"], out.dropped);
    read(value["clock"], out.clock);
    read(value["entries"], out.entries);
}

inline Value write(const ImperfectStatus& value) {
    Object out;
    out.push_back({"allowed", write(value.allowed)});
    out.push_back({"over_capacity", write(value.over_capacity)});
    out.push_back({"clone_imperfect", write(value.clone_imperfect)});
    return Value(std::move(out));
}

inline void read(const Value& value, ImperfectStatus& out) {
    read(value["allowed"], out.allowed);
    read(value["over_capacity"], out.over_capacity);
    read(value["clone_imperfect"], out.clone_imperfect);
}

inline Value write(const Bearing& value) {
    Object out;
    out.push_back({"window_ms", write(value.window_ms)});
    out.push_back({"mode", write(value.mode)});
    return Value(std::move(out));
}

inline void read(const Value& value, Bearing& out) {
    read(value["window_ms"], out.window_ms);
    read(value["mode"], out.mode);
}

inline Value write(const RenderStatus& value) {
    Object out;
    out.push_back({"mode", write(value.mode)});
    out.push_back({"full", write(value.full)});
    out.push_back({"ready", write(value.ready)});
    return Value(std::move(out));
}

inline void read(const Value& value, RenderStatus& out) {
    read(value["mode"], out.mode);
    read(value["full"], out.full);
    read(value["ready"], out.ready);
}

inline Value write(const SpreadStatus& value) {
    Object out;
    out.push_back({"percent", write(value.percent)});
    out.push_back({"span_us", write(value.span_us)});
    return Value(std::move(out));
}

inline void read(const Value& value, SpreadStatus& out) {
    read(value["percent"], out.percent);
    read(value["span_us"], out.span_us);
}

inline Value write(const CountersSnapshot& value) {
    Object out;
    out.push_back({"frames_tx", write(value.frames_tx)});
    out.push_back({"frames_rx", write(value.frames_rx)});
    out.push_back({"crc_drops", write(value.crc_drops)});
    out.push_back({"reconnects", write(value.reconnects)});
    out.push_back({"restarts", write(value.restarts)});
    return Value(std::move(out));
}

inline void read(const Value& value, CountersSnapshot& out) {
    read(value["frames_tx"], out.frames_tx);
    read(value["frames_rx"], out.frames_rx);
    read(value["crc_drops"], out.crc_drops);
    read(value["reconnects"], out.reconnects);
    read(value["restarts"], out.restarts);
}

inline Value write(const UsageSnapshot& value) {
    Object out;
    out.push_back({"class_", write(value.class_)});
    out.push_back({"direction", write(value.direction)});
    out.push_back({"usages", write(value.usages)});
    return Value(std::move(out));
}

inline void read(const Value& value, UsageSnapshot& out) {
    read(value["class_"], out.class_);
    read(value["direction"], out.direction);
    read(value["usages"], out.usages);
}

inline Value write(const TrafficEvent& value) {
    Object out;
    out.push_back({"class_", write(value.class_)});
    out.push_back({"id", write(value.id)});
    out.push_back({"direction", write(value.direction)});
    out.push_back({"flags", write(value.flags)});
    out.push_back({"true_len", write(value.true_len)});
    out.push_back({"bytes", write(value.bytes)});
    return Value(std::move(out));
}

inline void read(const Value& value, TrafficEvent& out) {
    read(value["class_"], out.class_);
    read(value["id"], out.id);
    read(value["direction"], out.direction);
    read(value["flags"], out.flags);
    read(value["true_len"], out.true_len);
    read(value["bytes"], out.bytes);
}

inline Value write(const BusEvent& value) {
    Object out;
    out.push_back({"kind", write(value.kind)});
    out.push_back({"configuration", write(value.configuration)});
    out.push_back({"interface", write(value.interface)});
    out.push_back({"alt", write(value.alt)});
    return Value(std::move(out));
}

inline void read(const Value& value, BusEvent& out) {
    read(value["kind"], out.kind);
    read(value["configuration"], out.configuration);
    read(value["interface"], out.interface);
    read(value["alt"], out.alt);
}

inline Value write(const MotionEvent& value) {
    Object out;
    out.push_back({"dx", write(value.dx)});
    out.push_back({"dy", write(value.dy)});
    out.push_back({"dz", write(value.dz)});
    out.push_back({"pan", write(value.pan)});
    return Value(std::move(out));
}

inline void read(const Value& value, MotionEvent& out) {
    read(value["dx"], out.dx);
    read(value["dy"], out.dy);
    read(value["dz"], out.dz);
    read(value["pan"], out.pan);
}

inline Value write(const Stamped& value) {
    Object out;
    out.push_back({"host_ns", write(value.host_ns)});
    out.push_back({"box_us", write(value.box_us)});
    out.push_back({"excess_ns", write(value.excess_ns)});
    return Value(std::move(out));
}

inline void read(const Value& value, Stamped& out) {
    read(value["host_ns"], out.host_ns);
    read(value["box_us"], out.box_us);
    read(value["excess_ns"], out.excess_ns);
}

inline Value write(const LogLine& value) {
    Object out;
    out.push_back({"level", write(value.level)});
    out.push_back({"text", write(value.text)});
    return Value(std::move(out));
}

inline void read(const Value& value, LogLine& out) {
    read(value["level"], out.level);
    read(value["text"], out.text);
}

// a generated function by its class and method, an empty optional for one that's written by hand
inline std::optional<Value> call(Context& ctx, const std::string& cls, const std::string& method, const Array& args) {
    if (cls == "ClipFrame" && method == "clear") {
        ctx.frame().clear();
        return Value();
    }
    if (cls == "ClipFrame" && method == "move") {
        ctx.frame().move(arg<std::int16_t>(args, 0), arg<std::int16_t>(args, 1));
        return Value();
    }
    if (cls == "ClipFrame" && method == "wheel") {
        ctx.frame().wheel(arg<std::int16_t>(args, 0));
        return Value();
    }
    if (cls == "ClipFrame" && method == "pan") {
        ctx.frame().pan(arg<std::int16_t>(args, 0));
        return Value();
    }
    if (cls == "ClipFrame" && method == "edge") {
        ctx.frame().edge(arg<Usage>(args, 0), arg<Action>(args, 1));
        return Value();
    }
    if (cls == "ClipFrame" && method == "press") {
        ctx.frame().press(arg<Usage>(args, 0));
        return Value();
    }
    if (cls == "ClipFrame" && method == "release") {
        ctx.frame().release(arg<Usage>(args, 0));
        return Value();
    }
    if (cls == "ClipFrame" && method == "force_release") {
        ctx.frame().force_release(arg<Usage>(args, 0));
        return Value();
    }
    if (cls == "ClipFrame" && method == "raw") {
        ctx.frame().raw(arg<std::uint8_t>(args, 0), arg<Direction>(args, 1), arg<std::vector<std::uint8_t>>(args, 2));
        return Value();
    }
    if (cls == "ClipFrame" && method == "transfer") {
        ctx.frame().transfer(arg<std::uint8_t>(args, 0), arg<Setup>(args, 1), arg<std::vector<std::uint8_t>>(args, 2));
        return Value();
    }
    if (cls == "ClipFrame" && method == "byte_len") {
        return write(ctx.frame().byte_len());
    }
    if (cls == "ClipBuilder" && method == "clear") {
        ctx.builder().clear();
        return Value();
    }
    if (cls == "ClipBuilder" && method == "byte_len") {
        return write(ctx.builder().byte_len());
    }
    if (cls == "ClipBuilder" && method == "gap") {
        ctx.builder().gap(arg<std::uint16_t>(args, 0));
        return Value();
    }
    if (cls == "ClipBuilder" && method == "move") {
        ctx.builder().move(arg<std::int16_t>(args, 0), arg<std::int16_t>(args, 1));
        return Value();
    }
    if (cls == "ClipBuilder" && method == "wheel") {
        ctx.builder().wheel(arg<std::int16_t>(args, 0));
        return Value();
    }
    if (cls == "ClipBuilder" && method == "pan") {
        ctx.builder().pan(arg<std::int16_t>(args, 0));
        return Value();
    }
    if (cls == "ClipBuilder" && method == "press") {
        ctx.builder().press(arg<Usage>(args, 0));
        return Value();
    }
    if (cls == "ClipBuilder" && method == "release") {
        ctx.builder().release(arg<Usage>(args, 0));
        return Value();
    }
    if (cls == "ClipBuilder" && method == "force_release") {
        ctx.builder().force_release(arg<Usage>(args, 0));
        return Value();
    }
    if (cls == "ClipBuilder" && method == "edge") {
        ctx.builder().edge(arg<Usage>(args, 0), arg<Action>(args, 1));
        return Value();
    }
    if (cls == "ClipBuilder" && method == "raw") {
        ctx.builder().raw(arg<std::uint8_t>(args, 0), arg<Direction>(args, 1), arg<std::vector<std::uint8_t>>(args, 2));
        return Value();
    }
    if (cls == "ClipBuilder" && method == "transfer") {
        ctx.builder().transfer(arg<std::uint8_t>(args, 0), arg<Setup>(args, 1), arg<std::vector<std::uint8_t>>(args, 2));
        return Value();
    }
    if (cls == "ClipBuilder" && method == "frame") {
        ctx.builder().frame(ctx.frame());
        return Value();
    }
    if (cls == "Clip" && method == "append") {
        ctx.clip().append(ctx.builder());
        return Value();
    }
    if (cls == "Clip" && method == "set_loop") {
        ctx.clip().set_loop(arg<bool>(args, 0));
        return Value();
    }
    if (cls == "Clip" && method == "set_retain") {
        ctx.clip().set_retain(arg<bool>(args, 0));
        return Value();
    }
    if (cls == "Clip" && method == "set_ride") {
        ctx.clip().set_ride(arg<bool>(args, 0));
        return Value();
    }
    if (cls == "Clip" && method == "bind") {
        ctx.clip().bind(arg<ClipTrigger>(args, 0));
        return Value();
    }
    if (cls == "Clip" && method == "unbind") {
        ctx.clip().unbind(arg<Usage>(args, 0), arg<Edge>(args, 1));
        return Value();
    }
    if (cls == "Clip" && method == "bind_packet") {
        ctx.clip().bind_packet(arg<ClipPacketTrigger>(args, 0));
        return Value();
    }
    if (cls == "Clip" && method == "clear_triggers") {
        ctx.clip().clear_triggers();
        return Value();
    }
    if (cls == "Clip" && method == "start") {
        ctx.clip().start();
        return Value();
    }
    if (cls == "Clip" && method == "stop") {
        ctx.clip().stop();
        return Value();
    }
    if (cls == "Clip" && method == "pause") {
        ctx.clip().pause();
        return Value();
    }
    if (cls == "Clip" && method == "resume") {
        ctx.clip().resume();
        return Value();
    }
    if (cls == "Clip" && method == "restart") {
        ctx.clip().restart();
        return Value();
    }
    if (cls == "Clip" && method == "toggle") {
        ctx.clip().toggle();
        return Value();
    }
    if (cls == "Clip" && method == "clear") {
        ctx.clip().clear();
        return Value();
    }
    if (cls == "Clip" && method == "finalize") {
        ctx.clip().finalize();
        return Value();
    }
    if (cls == "Clip" && method == "lost") {
        return write(ctx.clip().lost());
    }
    if (cls == "Clip" && method == "query_status") {
        return write(ctx.clip().query_status());
    }
    if (cls == "Clip" && method == "query_config") {
        return write(ctx.clip().query_config());
    }
    if (cls == "Device" && method == "move_rel") {
        ctx.device().move_rel(arg<std::int16_t>(args, 0), arg<std::int16_t>(args, 1));
        return Value();
    }
    if (cls == "Device" && method == "wheel") {
        ctx.device().wheel(arg<std::int16_t>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "move_rel_now") {
        ctx.device().move_rel_now(arg<std::int16_t>(args, 0), arg<std::int16_t>(args, 1));
        return Value();
    }
    if (cls == "Device" && method == "wheel_now") {
        ctx.device().wheel_now(arg<std::int16_t>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "pan") {
        ctx.device().pan(arg<std::int16_t>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "pan_now") {
        ctx.device().pan_now(arg<std::int16_t>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "flush_motion") {
        ctx.device().flush_motion();
        return Value();
    }
    if (cls == "Device" && method == "discard_motion") {
        ctx.device().discard_motion();
        return Value();
    }
    if (cls == "Device" && method == "move_axis") {
        ctx.device().move_axis(arg<Motion>(args, 0), arg<MoveTiming>(args, 1), arg<PendingMotion>(args, 2));
        return Value();
    }
    if (cls == "Device" && method == "inject") {
        ctx.device().inject(arg<Usage>(args, 0), arg<Action>(args, 1));
        return Value();
    }
    if (cls == "Device" && method == "press") {
        ctx.device().press(arg<Usage>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "soft_release") {
        ctx.device().soft_release(arg<Usage>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "force_release") {
        ctx.device().force_release(arg<Usage>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "scale") {
        ctx.device().scale(arg<LockTarget>(args, 0), arg<Direction>(args, 1), arg<std::int16_t>(args, 2));
        return Value();
    }
    if (cls == "Device" && method == "scale_all") {
        ctx.device().scale_all(arg<Blanket>(args, 0), arg<Direction>(args, 1), arg<std::int16_t>(args, 2));
        return Value();
    }
    if (cls == "Device" && method == "lock") {
        ctx.device().lock(arg<LockTarget>(args, 0), arg<Direction>(args, 1));
        return Value();
    }
    if (cls == "Device" && method == "unlock") {
        ctx.device().unlock(arg<LockTarget>(args, 0), arg<Direction>(args, 1));
        return Value();
    }
    if (cls == "Device" && method == "lock_all") {
        ctx.device().lock_all(arg<Blanket>(args, 0), arg<Direction>(args, 1));
        return Value();
    }
    if (cls == "Device" && method == "unlock_all") {
        ctx.device().unlock_all(arg<Blanket>(args, 0), arg<Direction>(args, 1));
        return Value();
    }
    if (cls == "Device" && method == "led") {
        ctx.device().led(arg<LedTarget>(args, 0), arg<LedMode>(args, 1), arg<std::uint8_t>(args, 2));
        return Value();
    }
    if (cls == "Device" && method == "reset") {
        ctx.device().reset();
        return Value();
    }
    if (cls == "Device" && method == "factory_reset") {
        ctx.device().factory_reset();
        return Value();
    }
    if (cls == "Device" && method == "reapply") {
        ctx.device().reapply();
        return Value();
    }
    if (cls == "Device" && method == "reconnect") {
        ctx.device().reconnect();
        return Value();
    }
    if (cls == "Device" && method == "reboot") {
        ctx.device().reboot(arg<RebootTarget>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "allow_imperfect_clones") {
        ctx.device().allow_imperfect_clones(arg<bool>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "raw") {
        ctx.device().raw(arg<std::uint8_t>(args, 0), arg<Direction>(args, 1), arg<std::vector<std::uint8_t>>(args, 2));
        return Value();
    }
    if (cls == "Device" && method == "set_rewrite") {
        ctx.device().set_rewrite(arg<RewriteRule>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "clear_rewrite") {
        ctx.device().clear_rewrite();
        return Value();
    }
    if (cls == "Device" && method == "query_rewrite") {
        return write(ctx.device().query_rewrite());
    }
    if (cls == "Device" && method == "query_rewrite_entry") {
        return write(ctx.device().query_rewrite_entry(arg<std::uint8_t>(args, 0)));
    }
    if (cls == "Device" && method == "set_patch") {
        ctx.device().set_patch(arg<Patch>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "apply_patch") {
        ctx.device().apply_patch();
        return Value();
    }
    if (cls == "Device" && method == "clear_patch") {
        ctx.device().clear_patch();
        return Value();
    }
    if (cls == "Device" && method == "query_patches") {
        return write(ctx.device().query_patches());
    }
    if (cls == "Device" && method == "query_patch_entry") {
        return write(ctx.device().query_patch_entry(arg<std::uint8_t>(args, 0)));
    }
    if (cls == "Device" && method == "transform") {
        ctx.device().transform(arg<Transform>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "untransform") {
        ctx.device().untransform(arg<Transform>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "clear_transforms") {
        ctx.device().clear_transforms();
        return Value();
    }
    if (cls == "Device" && method == "transform_swap") {
        ctx.device().transform_swap(arg<Axis>(args, 0), arg<Axis>(args, 1));
        return Value();
    }
    if (cls == "Device" && method == "transform_remap") {
        ctx.device().transform_remap(arg<LockTarget>(args, 0), arg<LockTarget>(args, 1));
        return Value();
    }
    if (cls == "Device" && method == "query_transforms") {
        return write(ctx.device().query_transforms());
    }
    if (cls == "Device" && method == "set_name") {
        ctx.device().set_name(arg<std::string>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "clear_name") {
        ctx.device().clear_name();
        return Value();
    }
    if (cls == "Device" && method == "set_render") {
        ctx.device().set_render(arg<RenderMode>(args, 0), arg<bool>(args, 1));
        return Value();
    }
    if (cls == "Device" && method == "set_spread") {
        ctx.device().set_spread(arg<std::uint16_t>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "query_version") {
        return write(ctx.device().query_version());
    }
    if (cls == "Device" && method == "firmware_info") {
        return write(ctx.device().firmware_info());
    }
    if (cls == "Device" && method == "abort_update") {
        ctx.device().abort_update(arg<UpdateTarget>(args, 0));
        return Value();
    }
    if (cls == "Device" && method == "activate_firmware") {
        ctx.device().activate_firmware();
        return Value();
    }
    if (cls == "Device" && method == "wait_firmware_confirmed") {
        return write(ctx.device().wait_firmware_confirmed());
    }
    if (cls == "Device" && method == "query_health") {
        return write(ctx.device().query_health());
    }
    if (cls == "Device" && method == "device_info") {
        return write(ctx.device().device_info());
    }
    if (cls == "Device" && method == "caps") {
        return write(ctx.device().caps());
    }
    if (cls == "Device" && method == "query_rate") {
        return write(ctx.device().query_rate());
    }
    if (cls == "Device" && method == "query_stats") {
        return write(ctx.device().query_stats());
    }
    if (cls == "Device" && method == "query_locks") {
        return write(ctx.device().query_locks());
    }
    if (cls == "Device" && method == "query_catch") {
        return write(ctx.device().query_catch());
    }
    if (cls == "Device" && method == "query_imperfect") {
        return write(ctx.device().query_imperfect());
    }
    if (cls == "Device" && method == "query_emit_pace") {
        return write(ctx.device().query_emit_pace());
    }
    if (cls == "Device" && method == "query_bearing") {
        return write(ctx.device().query_bearing());
    }
    if (cls == "Device" && method == "query_render") {
        return write(ctx.device().query_render());
    }
    if (cls == "Device" && method == "query_spread") {
        return write(ctx.device().query_spread());
    }
    if (cls == "Device" && method == "counters") {
        return write(ctx.device().counters());
    }
    if (cls == "" && method == "default_query_timeout_ms") {
        return write(anybox::medius::default_query_timeout_ms());
    }
    if (cls == "" && method == "default_transfer_timeout_ms") {
        return write(anybox::medius::default_transfer_timeout_ms());
    }
    if (cls == "" && method == "default_keepalive_cadence_ms") {
        return write(anybox::medius::default_keepalive_cadence_ms());
    }
    if (cls == "" && method == "abi_version") {
        return write(anybox::medius::abi_version());
    }
    if (cls == "" && method == "version") {
        return write(anybox::medius::version());
    }
    if (cls == "Timeline" && method == "reset") {
        ctx.timeline().reset(arg<ClockDomain>(args, 0));
        return Value();
    }
    if (cls == "Timeline" && method == "samples") {
        return write(ctx.timeline().samples(arg<ClockDomain>(args, 0)));
    }
    if (cls == "MockBox" && method == "set_version") {
        ctx.mock().set_version(arg<Version>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "set_health") {
        ctx.mock().set_health(arg<Health>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "set_device_info") {
        ctx.mock().set_device_info(arg<DeviceInfo>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "set_caps") {
        ctx.mock().set_caps(arg<Caps>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "set_mouse_caps") {
        ctx.mock().set_mouse_caps(arg<MouseCaps>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "set_kbd_caps") {
        ctx.mock().set_kbd_caps(arg<KbdCaps>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "set_rate") {
        ctx.mock().set_rate(arg<Rate>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "set_stats") {
        ctx.mock().set_stats(arg<Stats>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "set_locks") {
        ctx.mock().set_locks(arg<Locks>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "set_catch_state") {
        ctx.mock().set_catch_state(arg<CatchState>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "set_imperfect_status") {
        ctx.mock().set_imperfect_status(arg<ImperfectStatus>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "set_transfer_reply") {
        ctx.mock().set_transfer_reply(arg<TransferStatus>(args, 0), arg<std::vector<std::uint8_t>>(args, 1));
        return Value();
    }
    if (cls == "MockBox" && method == "set_render") {
        ctx.mock().set_render(arg<RenderMode>(args, 0), arg<bool>(args, 1), arg<bool>(args, 2));
        return Value();
    }
    if (cls == "MockBox" && method == "set_spread_learned") {
        ctx.mock().set_spread_learned(arg<std::uint32_t>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "set_advertised_hz") {
        ctx.mock().set_advertised_hz(arg<std::uint16_t>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "set_clip_status") {
        ctx.mock().set_clip_status(arg<ClipStatus>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "set_clip_settings") {
        ctx.mock().set_clip_settings(arg<ClipSettings>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "restart") {
        ctx.mock().restart();
        return Value();
    }
    if (cls == "MockBox" && method == "link_lost") {
        ctx.mock().link_lost();
        return Value();
    }
    if (cls == "MockBox" && method == "detach") {
        ctx.mock().detach(arg<bool>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "attach") {
        ctx.mock().attach();
        return Value();
    }
    if (cls == "MockBox" && method == "refuse_next_activate") {
        ctx.mock().refuse_next_activate(arg<std::uint8_t>(args, 0));
        return Value();
    }
    if (cls == "MockBox" && method == "reboot_after_activate") {
        ctx.mock().reboot_after_activate(arg<std::uint32_t>(args, 0), arg<std::uint32_t>(args, 1));
        return Value();
    }
    if (cls == "MockBox" && method == "silent") {
        ctx.mock().silent();
        return Value();
    }
    if (cls == "MockBox" && method == "push_log") {
        ctx.mock().push_log(arg<LogLevel>(args, 0), arg<std::string>(args, 1));
        return Value();
    }
    if (cls == "MockBox" && method == "push_motion") {
        ctx.mock().push_motion(arg<std::uint8_t>(args, 0), arg<std::uint32_t>(args, 1), arg<MotionEvent>(args, 2));
        return Value();
    }
    if (cls == "MockBox" && method == "push_usages") {
        ctx.mock().push_usages(arg<std::uint8_t>(args, 0), arg<std::uint32_t>(args, 1), arg<UsageSnapshot>(args, 2));
        return Value();
    }
    if (cls == "MockBox" && method == "push_traffic") {
        ctx.mock().push_traffic(arg<std::uint8_t>(args, 0), arg<std::uint32_t>(args, 1), arg<ClockDomain>(args, 2), arg<TrafficEvent>(args, 3));
        return Value();
    }
    if (cls == "MockBox" && method == "recorded") {
        return write(ctx.mock().recorded());
    }
    if (cls == "MockBox" && method == "saw") {
        return write(ctx.mock().saw(arg<FrameType>(args, 0)));
    }
    if (cls == "MockBox" && method == "clear_recorded") {
        ctx.mock().clear_recorded();
        return Value();
    }
    return std::nullopt;
}
