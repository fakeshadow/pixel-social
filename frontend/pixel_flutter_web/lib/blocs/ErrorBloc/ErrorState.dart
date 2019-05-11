import 'package:equatable/equatable.dart';
import 'package:flutter_web/widgets.dart';

abstract class ErrorState extends Equatable {
  ErrorState([List props = const []]) : super(props);
}

class NoSnack extends ErrorState {}

class Init extends ErrorState {}

class ShowError extends ErrorState {
  final String error;

  ShowError({@required this.error}) : super([error]);
}

class ShowSuccess extends ErrorState {
  final String success;

  ShowSuccess({@required this.success}) : super([success]);
}
