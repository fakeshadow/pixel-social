import 'package:bloc/bloc.dart';

import 'package:pixel_flutter/blocs/CategoryBloc/CategoryState.dart';
import 'package:pixel_flutter/blocs/CategoryBloc/CategoryEvent.dart';

import 'package:pixel_flutter/blocs/Repo/CategoryRepo.dart';

class CategoryBloc extends Bloc<CategoryEvent, CategoryState> {
  final CategoryRepo _categoryRepo = CategoryRepo();

  @override
  CategoryState get initialState => CategoryInit();

  Stream<CategoryState> mapEventToState(
    CategoryEvent event,
  ) async* {
    if (event is LoadCategories) {
      yield CategoryLoading();
      try {
        final _categories = await _categoryRepo.loadCategories();

        _categoryRepo.saveCategories(categories: _categories);
        yield CategoryLoaded(categories: _categories);
      } catch (e) {
        yield CategoryFailed(error: e);
      }
    }
  }
}
