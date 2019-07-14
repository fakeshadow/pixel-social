import 'package:sqflite/sqlite_api.dart';
import 'package:bloc/bloc.dart';

import 'package:pixel_flutter/blocs/ErrorBloc/ErrorBloc.dart';
import 'package:pixel_flutter/blocs/ErrorBloc/ErrorEvent.dart';
import 'package:pixel_flutter/blocs/CategoryBloc/CategoryState.dart';
import 'package:pixel_flutter/blocs/CategoryBloc/CategoryEvent.dart';

import 'package:pixel_flutter/blocs/Repo/CategoryRepo.dart';

class CategoryBloc extends Bloc<CategoryEvent, CategoryState> {

  final ErrorBloc errorBloc;
  final Database db;

  CategoryBloc({this.errorBloc, this.db});

  @override
  CategoryState get initialState => CategoryUnload();

  Stream<CategoryState> mapEventToState(
    CategoryEvent event,
  ) async* {
    if (event is LoadCategories) {
      yield CategoryLoading();
      try {
        final _categories = await CategoryRepo.loadCategories(db: db);
        yield CategoryLoaded(categories: _categories);
      } catch (e) {
        errorBloc.dispatch(GetError(error: e.toString()));
      }
    }
  }
}
