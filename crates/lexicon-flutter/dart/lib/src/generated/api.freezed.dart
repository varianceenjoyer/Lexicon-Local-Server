// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'api.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

T _$identity<T>(T value) => value;

final _privateConstructorUsedError = UnsupportedError(
    'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models');

/// @nodoc
mixin _$DownloadEvent {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(DownloadProgress field0) progress,
    required TResult Function(String modelId, String path) complete,
    required TResult Function(String modelId, String error) error,
    required TResult Function(String modelId) cancelled,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(DownloadProgress field0)? progress,
    TResult? Function(String modelId, String path)? complete,
    TResult? Function(String modelId, String error)? error,
    TResult? Function(String modelId)? cancelled,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(DownloadProgress field0)? progress,
    TResult Function(String modelId, String path)? complete,
    TResult Function(String modelId, String error)? error,
    TResult Function(String modelId)? cancelled,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(DownloadEvent_Progress value) progress,
    required TResult Function(DownloadEvent_Complete value) complete,
    required TResult Function(DownloadEvent_Error value) error,
    required TResult Function(DownloadEvent_Cancelled value) cancelled,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(DownloadEvent_Progress value)? progress,
    TResult? Function(DownloadEvent_Complete value)? complete,
    TResult? Function(DownloadEvent_Error value)? error,
    TResult? Function(DownloadEvent_Cancelled value)? cancelled,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(DownloadEvent_Progress value)? progress,
    TResult Function(DownloadEvent_Complete value)? complete,
    TResult Function(DownloadEvent_Error value)? error,
    TResult Function(DownloadEvent_Cancelled value)? cancelled,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $DownloadEventCopyWith<$Res> {
  factory $DownloadEventCopyWith(
          DownloadEvent value, $Res Function(DownloadEvent) then) =
      _$DownloadEventCopyWithImpl<$Res, DownloadEvent>;
}

/// @nodoc
class _$DownloadEventCopyWithImpl<$Res, $Val extends DownloadEvent>
    implements $DownloadEventCopyWith<$Res> {
  _$DownloadEventCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of DownloadEvent
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$DownloadEvent_ProgressImplCopyWith<$Res> {
  factory _$$DownloadEvent_ProgressImplCopyWith(
          _$DownloadEvent_ProgressImpl value,
          $Res Function(_$DownloadEvent_ProgressImpl) then) =
      __$$DownloadEvent_ProgressImplCopyWithImpl<$Res>;
  @useResult
  $Res call({DownloadProgress field0});
}

/// @nodoc
class __$$DownloadEvent_ProgressImplCopyWithImpl<$Res>
    extends _$DownloadEventCopyWithImpl<$Res, _$DownloadEvent_ProgressImpl>
    implements _$$DownloadEvent_ProgressImplCopyWith<$Res> {
  __$$DownloadEvent_ProgressImplCopyWithImpl(
      _$DownloadEvent_ProgressImpl _value,
      $Res Function(_$DownloadEvent_ProgressImpl) _then)
      : super(_value, _then);

  /// Create a copy of DownloadEvent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$DownloadEvent_ProgressImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as DownloadProgress,
    ));
  }
}

/// @nodoc

class _$DownloadEvent_ProgressImpl extends DownloadEvent_Progress {
  const _$DownloadEvent_ProgressImpl(this.field0) : super._();

  @override
  final DownloadProgress field0;

  @override
  String toString() {
    return 'DownloadEvent.progress(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$DownloadEvent_ProgressImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of DownloadEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$DownloadEvent_ProgressImplCopyWith<_$DownloadEvent_ProgressImpl>
      get copyWith => __$$DownloadEvent_ProgressImplCopyWithImpl<
          _$DownloadEvent_ProgressImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(DownloadProgress field0) progress,
    required TResult Function(String modelId, String path) complete,
    required TResult Function(String modelId, String error) error,
    required TResult Function(String modelId) cancelled,
  }) {
    return progress(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(DownloadProgress field0)? progress,
    TResult? Function(String modelId, String path)? complete,
    TResult? Function(String modelId, String error)? error,
    TResult? Function(String modelId)? cancelled,
  }) {
    return progress?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(DownloadProgress field0)? progress,
    TResult Function(String modelId, String path)? complete,
    TResult Function(String modelId, String error)? error,
    TResult Function(String modelId)? cancelled,
    required TResult orElse(),
  }) {
    if (progress != null) {
      return progress(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(DownloadEvent_Progress value) progress,
    required TResult Function(DownloadEvent_Complete value) complete,
    required TResult Function(DownloadEvent_Error value) error,
    required TResult Function(DownloadEvent_Cancelled value) cancelled,
  }) {
    return progress(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(DownloadEvent_Progress value)? progress,
    TResult? Function(DownloadEvent_Complete value)? complete,
    TResult? Function(DownloadEvent_Error value)? error,
    TResult? Function(DownloadEvent_Cancelled value)? cancelled,
  }) {
    return progress?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(DownloadEvent_Progress value)? progress,
    TResult Function(DownloadEvent_Complete value)? complete,
    TResult Function(DownloadEvent_Error value)? error,
    TResult Function(DownloadEvent_Cancelled value)? cancelled,
    required TResult orElse(),
  }) {
    if (progress != null) {
      return progress(this);
    }
    return orElse();
  }
}

abstract class DownloadEvent_Progress extends DownloadEvent {
  const factory DownloadEvent_Progress(final DownloadProgress field0) =
      _$DownloadEvent_ProgressImpl;
  const DownloadEvent_Progress._() : super._();

  DownloadProgress get field0;

  /// Create a copy of DownloadEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$DownloadEvent_ProgressImplCopyWith<_$DownloadEvent_ProgressImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$DownloadEvent_CompleteImplCopyWith<$Res> {
  factory _$$DownloadEvent_CompleteImplCopyWith(
          _$DownloadEvent_CompleteImpl value,
          $Res Function(_$DownloadEvent_CompleteImpl) then) =
      __$$DownloadEvent_CompleteImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String modelId, String path});
}

/// @nodoc
class __$$DownloadEvent_CompleteImplCopyWithImpl<$Res>
    extends _$DownloadEventCopyWithImpl<$Res, _$DownloadEvent_CompleteImpl>
    implements _$$DownloadEvent_CompleteImplCopyWith<$Res> {
  __$$DownloadEvent_CompleteImplCopyWithImpl(
      _$DownloadEvent_CompleteImpl _value,
      $Res Function(_$DownloadEvent_CompleteImpl) _then)
      : super(_value, _then);

  /// Create a copy of DownloadEvent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? modelId = null,
    Object? path = null,
  }) {
    return _then(_$DownloadEvent_CompleteImpl(
      modelId: null == modelId
          ? _value.modelId
          : modelId // ignore: cast_nullable_to_non_nullable
              as String,
      path: null == path
          ? _value.path
          : path // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$DownloadEvent_CompleteImpl extends DownloadEvent_Complete {
  const _$DownloadEvent_CompleteImpl(
      {required this.modelId, required this.path})
      : super._();

  @override
  final String modelId;
  @override
  final String path;

  @override
  String toString() {
    return 'DownloadEvent.complete(modelId: $modelId, path: $path)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$DownloadEvent_CompleteImpl &&
            (identical(other.modelId, modelId) || other.modelId == modelId) &&
            (identical(other.path, path) || other.path == path));
  }

  @override
  int get hashCode => Object.hash(runtimeType, modelId, path);

  /// Create a copy of DownloadEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$DownloadEvent_CompleteImplCopyWith<_$DownloadEvent_CompleteImpl>
      get copyWith => __$$DownloadEvent_CompleteImplCopyWithImpl<
          _$DownloadEvent_CompleteImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(DownloadProgress field0) progress,
    required TResult Function(String modelId, String path) complete,
    required TResult Function(String modelId, String error) error,
    required TResult Function(String modelId) cancelled,
  }) {
    return complete(modelId, path);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(DownloadProgress field0)? progress,
    TResult? Function(String modelId, String path)? complete,
    TResult? Function(String modelId, String error)? error,
    TResult? Function(String modelId)? cancelled,
  }) {
    return complete?.call(modelId, path);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(DownloadProgress field0)? progress,
    TResult Function(String modelId, String path)? complete,
    TResult Function(String modelId, String error)? error,
    TResult Function(String modelId)? cancelled,
    required TResult orElse(),
  }) {
    if (complete != null) {
      return complete(modelId, path);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(DownloadEvent_Progress value) progress,
    required TResult Function(DownloadEvent_Complete value) complete,
    required TResult Function(DownloadEvent_Error value) error,
    required TResult Function(DownloadEvent_Cancelled value) cancelled,
  }) {
    return complete(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(DownloadEvent_Progress value)? progress,
    TResult? Function(DownloadEvent_Complete value)? complete,
    TResult? Function(DownloadEvent_Error value)? error,
    TResult? Function(DownloadEvent_Cancelled value)? cancelled,
  }) {
    return complete?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(DownloadEvent_Progress value)? progress,
    TResult Function(DownloadEvent_Complete value)? complete,
    TResult Function(DownloadEvent_Error value)? error,
    TResult Function(DownloadEvent_Cancelled value)? cancelled,
    required TResult orElse(),
  }) {
    if (complete != null) {
      return complete(this);
    }
    return orElse();
  }
}

abstract class DownloadEvent_Complete extends DownloadEvent {
  const factory DownloadEvent_Complete(
      {required final String modelId,
      required final String path}) = _$DownloadEvent_CompleteImpl;
  const DownloadEvent_Complete._() : super._();

  String get modelId;
  String get path;

  /// Create a copy of DownloadEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$DownloadEvent_CompleteImplCopyWith<_$DownloadEvent_CompleteImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$DownloadEvent_ErrorImplCopyWith<$Res> {
  factory _$$DownloadEvent_ErrorImplCopyWith(_$DownloadEvent_ErrorImpl value,
          $Res Function(_$DownloadEvent_ErrorImpl) then) =
      __$$DownloadEvent_ErrorImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String modelId, String error});
}

/// @nodoc
class __$$DownloadEvent_ErrorImplCopyWithImpl<$Res>
    extends _$DownloadEventCopyWithImpl<$Res, _$DownloadEvent_ErrorImpl>
    implements _$$DownloadEvent_ErrorImplCopyWith<$Res> {
  __$$DownloadEvent_ErrorImplCopyWithImpl(_$DownloadEvent_ErrorImpl _value,
      $Res Function(_$DownloadEvent_ErrorImpl) _then)
      : super(_value, _then);

  /// Create a copy of DownloadEvent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? modelId = null,
    Object? error = null,
  }) {
    return _then(_$DownloadEvent_ErrorImpl(
      modelId: null == modelId
          ? _value.modelId
          : modelId // ignore: cast_nullable_to_non_nullable
              as String,
      error: null == error
          ? _value.error
          : error // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$DownloadEvent_ErrorImpl extends DownloadEvent_Error {
  const _$DownloadEvent_ErrorImpl({required this.modelId, required this.error})
      : super._();

  @override
  final String modelId;
  @override
  final String error;

  @override
  String toString() {
    return 'DownloadEvent.error(modelId: $modelId, error: $error)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$DownloadEvent_ErrorImpl &&
            (identical(other.modelId, modelId) || other.modelId == modelId) &&
            (identical(other.error, error) || other.error == error));
  }

  @override
  int get hashCode => Object.hash(runtimeType, modelId, error);

  /// Create a copy of DownloadEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$DownloadEvent_ErrorImplCopyWith<_$DownloadEvent_ErrorImpl> get copyWith =>
      __$$DownloadEvent_ErrorImplCopyWithImpl<_$DownloadEvent_ErrorImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(DownloadProgress field0) progress,
    required TResult Function(String modelId, String path) complete,
    required TResult Function(String modelId, String error) error,
    required TResult Function(String modelId) cancelled,
  }) {
    return error(modelId, this.error);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(DownloadProgress field0)? progress,
    TResult? Function(String modelId, String path)? complete,
    TResult? Function(String modelId, String error)? error,
    TResult? Function(String modelId)? cancelled,
  }) {
    return error?.call(modelId, this.error);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(DownloadProgress field0)? progress,
    TResult Function(String modelId, String path)? complete,
    TResult Function(String modelId, String error)? error,
    TResult Function(String modelId)? cancelled,
    required TResult orElse(),
  }) {
    if (error != null) {
      return error(modelId, this.error);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(DownloadEvent_Progress value) progress,
    required TResult Function(DownloadEvent_Complete value) complete,
    required TResult Function(DownloadEvent_Error value) error,
    required TResult Function(DownloadEvent_Cancelled value) cancelled,
  }) {
    return error(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(DownloadEvent_Progress value)? progress,
    TResult? Function(DownloadEvent_Complete value)? complete,
    TResult? Function(DownloadEvent_Error value)? error,
    TResult? Function(DownloadEvent_Cancelled value)? cancelled,
  }) {
    return error?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(DownloadEvent_Progress value)? progress,
    TResult Function(DownloadEvent_Complete value)? complete,
    TResult Function(DownloadEvent_Error value)? error,
    TResult Function(DownloadEvent_Cancelled value)? cancelled,
    required TResult orElse(),
  }) {
    if (error != null) {
      return error(this);
    }
    return orElse();
  }
}

abstract class DownloadEvent_Error extends DownloadEvent {
  const factory DownloadEvent_Error(
      {required final String modelId,
      required final String error}) = _$DownloadEvent_ErrorImpl;
  const DownloadEvent_Error._() : super._();

  String get modelId;
  String get error;

  /// Create a copy of DownloadEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$DownloadEvent_ErrorImplCopyWith<_$DownloadEvent_ErrorImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$DownloadEvent_CancelledImplCopyWith<$Res> {
  factory _$$DownloadEvent_CancelledImplCopyWith(
          _$DownloadEvent_CancelledImpl value,
          $Res Function(_$DownloadEvent_CancelledImpl) then) =
      __$$DownloadEvent_CancelledImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String modelId});
}

/// @nodoc
class __$$DownloadEvent_CancelledImplCopyWithImpl<$Res>
    extends _$DownloadEventCopyWithImpl<$Res, _$DownloadEvent_CancelledImpl>
    implements _$$DownloadEvent_CancelledImplCopyWith<$Res> {
  __$$DownloadEvent_CancelledImplCopyWithImpl(
      _$DownloadEvent_CancelledImpl _value,
      $Res Function(_$DownloadEvent_CancelledImpl) _then)
      : super(_value, _then);

  /// Create a copy of DownloadEvent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? modelId = null,
  }) {
    return _then(_$DownloadEvent_CancelledImpl(
      modelId: null == modelId
          ? _value.modelId
          : modelId // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$DownloadEvent_CancelledImpl extends DownloadEvent_Cancelled {
  const _$DownloadEvent_CancelledImpl({required this.modelId}) : super._();

  @override
  final String modelId;

  @override
  String toString() {
    return 'DownloadEvent.cancelled(modelId: $modelId)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$DownloadEvent_CancelledImpl &&
            (identical(other.modelId, modelId) || other.modelId == modelId));
  }

  @override
  int get hashCode => Object.hash(runtimeType, modelId);

  /// Create a copy of DownloadEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$DownloadEvent_CancelledImplCopyWith<_$DownloadEvent_CancelledImpl>
      get copyWith => __$$DownloadEvent_CancelledImplCopyWithImpl<
          _$DownloadEvent_CancelledImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(DownloadProgress field0) progress,
    required TResult Function(String modelId, String path) complete,
    required TResult Function(String modelId, String error) error,
    required TResult Function(String modelId) cancelled,
  }) {
    return cancelled(modelId);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(DownloadProgress field0)? progress,
    TResult? Function(String modelId, String path)? complete,
    TResult? Function(String modelId, String error)? error,
    TResult? Function(String modelId)? cancelled,
  }) {
    return cancelled?.call(modelId);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(DownloadProgress field0)? progress,
    TResult Function(String modelId, String path)? complete,
    TResult Function(String modelId, String error)? error,
    TResult Function(String modelId)? cancelled,
    required TResult orElse(),
  }) {
    if (cancelled != null) {
      return cancelled(modelId);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(DownloadEvent_Progress value) progress,
    required TResult Function(DownloadEvent_Complete value) complete,
    required TResult Function(DownloadEvent_Error value) error,
    required TResult Function(DownloadEvent_Cancelled value) cancelled,
  }) {
    return cancelled(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(DownloadEvent_Progress value)? progress,
    TResult? Function(DownloadEvent_Complete value)? complete,
    TResult? Function(DownloadEvent_Error value)? error,
    TResult? Function(DownloadEvent_Cancelled value)? cancelled,
  }) {
    return cancelled?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(DownloadEvent_Progress value)? progress,
    TResult Function(DownloadEvent_Complete value)? complete,
    TResult Function(DownloadEvent_Error value)? error,
    TResult Function(DownloadEvent_Cancelled value)? cancelled,
    required TResult orElse(),
  }) {
    if (cancelled != null) {
      return cancelled(this);
    }
    return orElse();
  }
}

abstract class DownloadEvent_Cancelled extends DownloadEvent {
  const factory DownloadEvent_Cancelled({required final String modelId}) =
      _$DownloadEvent_CancelledImpl;
  const DownloadEvent_Cancelled._() : super._();

  String get modelId;

  /// Create a copy of DownloadEvent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$DownloadEvent_CancelledImplCopyWith<_$DownloadEvent_CancelledImpl>
      get copyWith => throw _privateConstructorUsedError;
}
