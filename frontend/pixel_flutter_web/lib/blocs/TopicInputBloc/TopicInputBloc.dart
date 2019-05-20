import 'package:bloc/bloc.dart';

import 'package:pixel_flutter_web/blocs/TopicInputBlocs.dart';

/// use for both topic and post input
class TopicInputBloc extends Bloc<TopicInputEvent, TopicInputState> {
  @override
  TopicInputState get initialState => TopicInputState.Init();

  @override
  Stream<TopicInputState> mapEventToState(
    TopicInputEvent event,
  ) async* {
    if (event is TitleChanged) {
      yield currentState.copyWith(
        title: event.title,
        isTitleValid: _isTitleValid(event.title),
      );
    }

    if (event is BodyChanged) {
      yield currentState.copyWith(
        body: event.body,
        isBodyValid: _isBodyValid(event.body),
      );
    }
  }

  bool _isTitleValid(String title) {
    return title.length > 8;
  }

  bool _isBodyValid(String body) {
    return body.length > 8;
  }
}
