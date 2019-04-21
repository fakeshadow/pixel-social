import 'package:bloc/bloc.dart';
import 'package:pixel_flutter/blocs/VerticalTabBlocs.dart';

class VerticalTabBloc extends Bloc<VerticalTabEvent,VerticalTabState> {
  @override
  VerticalTabState get initialState => Selected(index: 1);

  Stream<VerticalTabState> mapEventToState(
      VerticalTabEvent event,
      ) async* {
    if (event is Tapped) {
      yield Selected(index: event.index);
    }
  }
}