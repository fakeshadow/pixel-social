import 'package:flutter_web/material.dart';

import 'package:pixel_flutter_web/env.dart';

import 'package:pixel_flutter_web/blocs/TopicsBlocs.dart';

import 'package:pixel_flutter_web/components/FloatingAppBar.dart';
import 'package:pixel_flutter_web/components/BasicLayout.dart';
import 'package:pixel_flutter_web/components/TopicsList.dart';
import 'package:pixel_flutter_web/components/SideMenu.dart';

import 'package:pixel_flutter_web/models/Category.dart';

class TopicsPage extends StatefulWidget with env{
  TopicsPage({Key key, this.category}) : super(key: key);

  final Category category;

  @override
  _TopicsPageState createState() => _TopicsPageState();
}

class _TopicsPageState extends State<TopicsPage> {
  TopicsBloc _topicsBloc;
  Category category;
  final _scrollController = ScrollController();
  final _scrollThreshold = 300.0;

  @override
  void initState() {
    category = widget.category;
    _topicsBloc = TopicsBloc();
    _topicsBloc.dispatch(GetTopics(categoryId: category.id));
    _scrollController.addListener(_onScroll);
    super.initState();
  }

  @override
  void dispose() {
    _topicsBloc.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return BasicLayout(
      scrollView: scrollView(_scrollController),
      sideMenu: SideMenu(),
    );
  }

  Widget scrollView(scrollController) {
    return Scrollbar(
      child: CustomScrollView(controller: scrollController, slivers: [
        FloatingAppBar(title: category.name),
        SliverPadding(
          padding: EdgeInsets.only(
            left: MediaQuery.of(context).size.width > widget.BREAK_POINT_WIDTH
                ? MediaQuery.of(context).size.width * 0.2
                : 0,
            right: MediaQuery.of(context).size.width > widget.BREAK_POINT_WIDTH_SM
                ? MediaQuery.of(context).size.width * 0.4
                : 0,
          ),
          sliver: TopicsList(
            topicsBloc: _topicsBloc,
          ),
        )
      ]),
    );
  }

  void _onScroll() {
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      _topicsBloc.dispatch(GetTopics(categoryId: 1));
    }
  }
}
