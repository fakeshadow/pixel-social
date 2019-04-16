import 'package:equatable/equatable.dart';
import 'package:flutter/widgets.dart';

abstract class ErrorEvent extends Equatable {
  ErrorEvent([List props = const []]) : super(props);
}

class GetError extends ErrorEvent {
  final String error;

  GetError({@required this.error}) : super([error]);
}
