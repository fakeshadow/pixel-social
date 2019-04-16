import 'package:bloc/bloc.dart';
import 'package:pixel_flutter/blocs/ErrorBlocs.dart';

class ErrorBloc extends Bloc<ErrorEvent, ErrorState> {
  @override
  ErrorState get initialState => NoError();

  Stream<ErrorState> mapEventToState(
    ErrorEvent event,
  ) async* {
    if (event is GetError) {
      yield ShowError(error: event.error);
      Duration(seconds: 3);
      yield NoError();
    }
  }
}
