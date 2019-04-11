import 'package:bloc/bloc.dart';
import 'package:flutter/widgets.dart';
import 'package:rxdart/rxdart.dart';

import 'package:pixel_flutter/blocs/CategoryBloc/CategoryState.dart';
import 'package:pixel_flutter/blocs/CategoryBloc/CategoryEvent.dart';

import 'package:pixel_flutter/blocs/CategoryBloc/CategoryRepo.dart';

class CategoryBloc extends Bloc<CategoryEvent, CategoryState> {
    final CategoryRepo _categoryRepo = CategoryRepo();

    @override
    CategoryState get initialState => CategoryLoading();

    Stream<CategoryState> mapEventToState(
        CategoryEvent event,
        ) async* {
      if (event is GetCategories) {
        yield* _mapGetCategories();
      }
    }

    Stream<CategoryState> _mapGetCategories() async* {
      try {
        final categories = await _categoryRepo.fetchCategories();
        print(categories);
        yield CategoryLoaded(categories);
      } catch (_) {
        yield CategoryFailed();
      }
    }

}