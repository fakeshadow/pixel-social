import 'package:equatable/equatable.dart';

abstract class EnvState extends Equatable {
  EnvState([List props = const []]) : super(props);
}

class NoEnv extends EnvState {}

class HaveEnv extends EnvState {
  final String url;

  HaveEnv({this.url}) : super([url]);
}
