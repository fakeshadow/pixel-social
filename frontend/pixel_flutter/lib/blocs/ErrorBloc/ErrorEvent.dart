import 'package:equatable/equatable.dart';
import 'package:flutter/widgets.dart';

abstract class ErrorEvent extends Equatable {
  ErrorEvent([List props = const []]) : super(props);
}

class HideSnack extends ErrorEvent {}

class GetError extends ErrorEvent {
  final String error;

  GetError({@required this.error}) : super([error]);
}

class GetSuccess extends ErrorEvent {
  final String success;

  GetSuccess({@required this.success}) : super([success]);
}

