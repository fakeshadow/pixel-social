import 'package:bloc/bloc.dart';
import 'package:pixel_flutter/blocs/HorizontalTabBlocs.dart';

class HorizontalTabBloc extends Bloc<HorizontalTabEvent,HorizontalTabState> {
  @override
  HorizontalTabState get initialState => Selected(index: 1);

  Stream<HorizontalTabState> mapEventToState(
      HorizontalTabEvent event,
      ) async* {
    if (event is Tapped) {
      yield Selected(index: event.index);
    }
  }
}