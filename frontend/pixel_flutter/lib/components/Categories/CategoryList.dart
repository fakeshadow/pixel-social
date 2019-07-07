import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:pixel_flutter/blocs/CategoryBlocs.dart';
import 'package:pixel_flutter/components/Categories/CategoryCard.dart';

class CategoryList extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final _catbloc = BlocProvider.of<CategoryBloc>(context);
    return BlocBuilder(
      bloc: _catbloc,
      builder: (BuildContext context, CategoryState state) {
        if (state is CategoryInit) {
          _catbloc.dispatch(LoadCategories());
        }
        if (state is CategoryLoaded) {
          return ListView.builder(
            itemBuilder: (BuildContext context, int index) {
              return CategoryCard(category: state.categories[index]);
            },
            scrollDirection: Axis.horizontal,
            itemCount: state.categories.length,
          );
        }
        if (state is CategoryLoading) {
          return Container(child: Center(child: CircularProgressIndicator()));
        }
        return Container();
      },
    );
  }
}
