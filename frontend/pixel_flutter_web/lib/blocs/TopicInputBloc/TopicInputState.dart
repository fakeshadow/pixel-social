import 'package:equatable/equatable.dart';
import 'package:meta/meta.dart';

@immutable
class TopicInputState extends Equatable {
  final String title, body;
  final bool isTitleValid, isBodyValid;

  bool get isTopicValid => isTitleValid && isBodyValid;

  bool get isPostValid => isBodyValid;

  TopicInputState({
    @required this.title,
    @required this.body,
    @required this.isTitleValid,
    @required this.isBodyValid,
  }) : super([
          title,
          body,
          isTitleValid,
          isBodyValid,
        ]);

  factory TopicInputState.Init() {
    return TopicInputState(
      title: '',
      body: '',
      isTitleValid: false,
      isBodyValid: false,
    );
  }

  TopicInputState copyWith({
    String title,
    String body,
    bool isTitleValid,
    bool isBodyValid,
  }) {
    return TopicInputState(
        title: title ?? this.title,
        body: body ?? this.body,
        isTitleValid: isTitleValid ?? this.isTitleValid,
        isBodyValid: isBodyValid ?? this.isBodyValid);
  }
}
