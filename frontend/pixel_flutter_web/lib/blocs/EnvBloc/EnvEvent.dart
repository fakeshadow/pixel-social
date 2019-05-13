import 'package:equatable/equatable.dart';

abstract class EnvEvent extends Equatable {
  EnvEvent([List props = const []]) : super(props);
}

class LoadEnv extends EnvEvent {
  final String url;

  LoadEnv({this.url});
}
