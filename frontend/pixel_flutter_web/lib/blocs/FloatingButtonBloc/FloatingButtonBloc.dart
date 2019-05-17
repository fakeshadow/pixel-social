import 'package:bloc/bloc.dart';
import 'package:pixel_flutter_web/blocs/FloatingButtonBlocs.dart';

// ToDo : use as genricl visible controller between widgets
class FloatingButtonBloc
    extends Bloc<FloatingButtonEvent, FloatingButtonState> {
  @override
  FloatingButtonState get initialState => IsVisible(isVisible: false);

  Stream<FloatingButtonState> mapEventToState(
    FloatingButtonEvent event,
  ) async* {
    if (event is ShowFloating) {
      yield IsVisible(isVisible: event.showFloating);
    }
  }
}
