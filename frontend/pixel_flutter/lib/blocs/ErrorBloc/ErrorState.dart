import 'package:equatable/equatable.dart';
import 'package:flutter/widgets.dart';

abstract class ErrorState extends Equatable {
  ErrorState([List props = const []]) : super(props);
}

class NoError extends ErrorState {}

class ShowError extends ErrorState {
  final String error;

  ShowError({@required this.error}) : super([error]);
}
