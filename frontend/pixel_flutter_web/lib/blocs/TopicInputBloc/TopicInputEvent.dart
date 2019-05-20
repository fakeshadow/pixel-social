import 'package:equatable/equatable.dart';
import 'package:meta/meta.dart';

@immutable
abstract class TopicInputEvent extends Equatable {
  TopicInputEvent([List props = const []]) : super(props);
}

class TitleChanged extends TopicInputEvent {
  final String title;

  TitleChanged({@required this.title}) : super([title]);
}

class BodyChanged extends TopicInputEvent {
  final String body;

  BodyChanged({@required this.body}) : super([body]);
}

class FormReset extends TopicInputEvent {}