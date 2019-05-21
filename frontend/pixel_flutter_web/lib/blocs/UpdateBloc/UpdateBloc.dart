import 'package:bloc/bloc.dart';
import 'package:rxdart/rxdart.dart';

import 'package:pixel_flutter_web/blocs/UpdateBlocs.dart';

import 'package:pixel_flutter_web/blocs/Repo/TopicsRepo.dart';
import 'package:pixel_flutter_web/blocs/Repo/UserRepo.dart';

import 'package:pixel_flutter_web/models/Topic.dart';

class UpdateBloc extends Bloc<UpdateEvent, UpdateState> {
  final topicsRepo = TopicsRepo();
  final userRepo = UserRepo();

  @override
  Stream<UpdateState> transform(Stream<UpdateEvent> events,
      Stream<UpdateState> Function(UpdateEvent event) next) {
    return super.transform(
      (events as Observable<UpdateEvent>)
          .debounceTime(Duration(milliseconds: 500)),
      next,
    );
  }

  @override
  get initialState => Updated();

  @override
  Stream<UpdateState> mapEventToState(
    UpdateEvent event,
  ) async* {
    if (event is AddTopic) {
      try {
        yield Updating();
        final _token = await userRepo.getLocal(key: 'token');
        final _topic = await topicsRepo.addTopic(
            Topic(
                title: event.title,
                body: event.body,
                categoryId: event.categoryId,
                thumbnail: event.thumbnail),
            _token);
        /// only yield topic id and topic title as for now we only use these two props for topic page construct
        yield GotTopic(topic: Topic(id: _topic.id, title: _topic.title));
      } catch (e) {
        yield GotError(error: e.toString());
      }
    }
  }
}
