import 'package:bloc/bloc.dart';
import 'package:pixel_flutter_web/blocs/EnvBlocs.dart';


class EnvBloc extends Bloc<EnvEvent, EnvState> {
  @override
  EnvState get initialState => NoEnv();

  Stream<EnvState> mapEventToState(EnvEvent event) async* {
    if (event is LoadEnv) {
      yield HaveEnv(url: event.url);
    }
  }
}
