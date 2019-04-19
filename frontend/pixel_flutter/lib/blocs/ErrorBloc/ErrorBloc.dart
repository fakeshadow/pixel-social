import 'package:bloc/bloc.dart';
import 'package:pixel_flutter/blocs/ErrorBlocs.dart';

/// error bloc is also used to show success snacks
class ErrorBloc extends Bloc<ErrorEvent, ErrorState> {
  @override
  ErrorState get initialState => NoSnack();

  Stream<ErrorState> mapEventToState(
    ErrorEvent event,
  ) async* {
    if (event is HideSnack) {
      yield NoSnack();
    }

    if (event is GetSuccess) {
      yield ShowSuccess(success: event.success);
      yield NoSnack();
    }

    if (event is GetError) {
      yield ShowError(error: event.error);
      yield NoSnack();
    }
  }
}
